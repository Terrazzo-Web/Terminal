use scopeguard::defer;
use scopeguard::guard;
use terrazzo::prelude::*;
use terrazzo::widgets::resize_event::ResizeEvent;
use tracing::Instrument as _;
use tracing::Span;
use tracing::debug;
use tracing::debug_span;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use wasm_bindgen::JsValue;

use super::TerminalsState;
use super::javascript::TerminalJs;
use super::terminal_tab::TerminalTab;
use crate::api;
use crate::api::TabTitle;
use crate::api::TerminalAddress;

const XTERMJS_ATTR: &str = "data-xtermjs";
const IS_ATTACHED: &str = "Y";

pub fn attach(template: XTemplate, terminal_tab: TerminalTab, state: TerminalsState) -> Consumers {
    let terminal = terminal_tab.address.to_owned();
    let terminal_id = terminal.id.clone();
    let _span = info_span!("XTermJS", %terminal_id).entered();
    let element = template.element();
    if let Some(IS_ATTACHED) = element.get_attribute(XTERMJS_ATTR).as_deref() {
        if terminal_tab.selected.get_value_untracked() {
            if let Some(xtermjs) = terminal_tab
                .xtermjs
                .lock()
                .or_throw("xtermjs.lock()")
                .clone()
            {
                debug!("Focus and fit size");
                xtermjs.focus();
                xtermjs.fit();
            }
        }
        return Consumers::default();
    }
    element
        .set_attribute(XTERMJS_ATTR, IS_ATTACHED)
        .or_throw(XTERMJS_ATTR);

    info!("Attaching XtermJS");
    let xtermjs = TerminalJs::new();
    *terminal_tab.xtermjs.lock().or_throw("xtermjs") = Some(xtermjs.clone());
    let xtermjs = guard(xtermjs, |xtermjs| xtermjs.dispose());
    xtermjs.open(&element);
    let on_data = xtermjs.do_on_data(terminal.clone());
    let on_resize = xtermjs.do_on_resize(terminal.clone());
    let on_title_change = xtermjs.do_on_title_change(terminal_tab.title.clone());
    let io = async move {
        let _on_data = on_data;
        let _on_resize = on_resize;
        let _on_title_change = on_title_change;
        let read_loop = xtermjs.read_loop(&terminal, state);
        let unsubscribe_resize_event = ResizeEvent::signal().add_subscriber({
            let xtermjs = xtermjs.clone();
            move |_| xtermjs.fit()
        });
        if terminal_tab.selected.get_value_untracked() {
            xtermjs.focus();
        }
        let () = read_loop.await;
        drop(unsubscribe_resize_event);
        drop(xtermjs);
        info!("Detached XtermJS");
    };
    wasm_bindgen_futures::spawn_local(io.in_current_span());
    return Consumers::default();
}

impl TerminalJs {
    fn do_on_data(&self, terminal: TerminalAddress) -> Closure<dyn FnMut(JsValue)> {
        let span = Span::current();
        let on_data: Closure<dyn FnMut(JsValue)> = Closure::new(move |data: JsValue| {
            let data = data.as_string().unwrap_or_default();
            let terminal = terminal.clone();
            let send = async move {
                let result = api::client::stream::write::write(terminal.clone(), data).await;
                // The channel is unbounded, the only possible error is the write_loop has dropped.
                return result.unwrap_or_else(|error| warn!("Write failed: {error}"));
            };
            wasm_bindgen_futures::spawn_local(send.instrument(span.clone()));
        });
        self.on_data(&on_data);
        return on_data;
    }

    fn do_on_resize(&self, terminal: TerminalAddress) -> Closure<dyn FnMut(JsValue)> {
        let span = Span::current();
        let this = self.clone();
        let mut first_resize = true;
        let on_resize: Closure<dyn FnMut(JsValue)> = Closure::new(move |data| {
            let _span = span.enter();
            let first_resize = std::mem::replace(&mut first_resize, false);
            debug!("Resize: {data:?} first_resize:{first_resize}");
            let resize = this.clone().do_resize(terminal.clone(), first_resize);
            wasm_bindgen_futures::spawn_local(resize.in_current_span());
        });
        self.on_resize(&on_resize);
        return on_resize;
    }

    async fn do_resize(self, terminal: TerminalAddress, force: bool) {
        let size = api::Size {
            rows: self.rows().as_f64().or_throw("rows") as i32,
            cols: self.cols().as_f64().or_throw("cols") as i32,
        };
        if let Err(error) = api::client::resize::resize(&terminal, size, force).await {
            warn!("Failed to resize: {error}");
        }
    }

    fn do_on_title_change(&self, title: XSignal<TabTitle<XString>>) -> Closure<dyn FnMut(JsValue)> {
        let span = Span::current();
        let on_title_change: Closure<dyn FnMut(JsValue)> = Closure::new(move |data: JsValue| {
            let _span = span.enter();
            info!("Title changed: {data:?}");
            if let Some(new_title) = data.as_string() {
                title.update_mut(|t| TabTitle {
                    shell_title: new_title.into(),
                    override_title: t.override_title.take(),
                });
            }
        });
        self.on_title_change(&on_title_change);
        return on_title_change;
    }

    async fn read_loop(&self, terminal: &TerminalAddress, state: TerminalsState) {
        async {
            debug!("Start");
            defer! { state.on_eos(&terminal.id); }
            self.fit();
            let eos = api::client::stream::read::read(terminal, |data| self.send(data)).await;
            match eos {
                Ok(()) => info!("End"),
                Err(error) => warn!("Failed: {error}"),
            }
        }
        .instrument(debug_span!("ReadLoop"))
        .await
    }
}
