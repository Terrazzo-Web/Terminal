use std::future::ready;

use futures::FutureExt as _;
use futures::SinkExt as _;
use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::select;
use scopeguard::defer;
use scopeguard::guard;
use terrazzo::prelude::*;
use terrazzo::widgets::resize_event::ResizeEvent;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use self::diagnostics::debug_span;
use self::diagnostics::error;
use self::diagnostics::info;
use self::diagnostics::info_span;
use self::diagnostics::span::Span;
use self::diagnostics::warn;
use super::TerminalsState;
use super::javascript::TerminalJs;
use super::terminal_tab::TerminalTab;
use crate::api::client::terminal_api;
use crate::api::shared::terminal_schema;
use crate::api::shared::terminal_schema::TabTitle;
use crate::api::shared::terminal_schema::TerminalAddress;
use crate::api::shared::terminal_schema::TerminalDef;

const XTERMJS_ATTR: &str = "data-xtermjs";
const IS_ATTACHED: &str = "Y";

pub fn attach(template: XTemplate, state: TerminalsState, terminal_tab: TerminalTab) -> Consumers {
    let terminal = terminal_tab.address.to_owned();
    let terminal_id = terminal.id.clone();
    let terminal_def = terminal_tab.to_terminal_def();
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
    let (input_tx, input_rx) = mpsc::unbounded();
    let on_data = xtermjs.do_on_data(input_tx);
    let on_resize = xtermjs.do_on_resize(terminal.clone());
    let on_title_change = xtermjs.do_on_title_change(terminal_tab.title.clone());
    let io = async move {
        let _on_data = on_data;
        let _on_resize = on_resize;
        let _on_title_change = on_title_change;
        let stream_loop = xtermjs.stream_loop(state, terminal_def, element);
        let write_loop = write_loop(&terminal, input_rx);
        let unsubscribe_resize_event = ResizeEvent::signal().add_subscriber({
            let xtermjs = xtermjs.clone();
            move |_| xtermjs.fit()
        });
        if terminal_tab.selected.get_value_untracked() {
            xtermjs.focus();
        }
        // TODO: If write fails, we should not close the tab
        select! {
            () = stream_loop.fuse() => info!("Stream loop closed"),
            () = write_loop.fuse() => info!("Write loop closed"),
        };
        drop(unsubscribe_resize_event);
        drop(xtermjs);
        info!("Detached XtermJS");
    };
    spawn_local(io.in_current_span());
    return Consumers::default();
}

impl TerminalJs {
    fn do_on_data(&self, input_tx: mpsc::UnboundedSender<String>) -> Closure<dyn FnMut(JsValue)> {
        let span = Span::current();
        let on_data: Closure<dyn FnMut(JsValue)> = Closure::new(move |data: JsValue| {
            let mut input_tx = input_tx.clone();
            let data = data.as_string().unwrap_or_default();
            let send = async move {
                let result = input_tx.send(data).await;
                // The channel is unbounded, the only possible error is the write_loop has dropped.
                return result.unwrap_or_else(|_| info!("Terminal closed"));
            };
            spawn_local(send.instrument(span.clone()));
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
            spawn_local(resize.in_current_span());
        });
        self.on_resize(&on_resize);
        return on_resize;
    }

    async fn do_resize(self, terminal: TerminalAddress, force: bool) {
        let size = terminal_schema::Size {
            rows: self.rows().as_f64().or_throw("rows") as i32,
            cols: self.cols().as_f64().or_throw("cols") as i32,
        };
        if let Err(error) = terminal_api::resize::resize(&terminal, size, force).await {
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

    async fn stream_loop(
        &self,
        state: TerminalsState,
        terminal_def: TerminalDef,
        element: Element,
    ) {
        async {
            debug!("Start");
            let on_init = || {
                self.fit();
                ready(())
            };
            let eos = terminal_api::stream::stream(state, terminal_def, element, on_init, |data| {
                self.send(data)
            })
            .await;
            match eos {
                Ok(()) => info!("End"),
                Err(error) => warn!("Failed: {error}"),
            }
        }
        .instrument(debug_span!("StreamLoop"))
        .await
    }
}

async fn write_loop(terminal: &TerminalAddress, input_rx: mpsc::UnboundedReceiver<String>) {
    async {
        defer!(debug!("End"));
        debug!("Start");
        let mut input_rx = input_rx.ready_chunks(10);
        while let Some(data) = &input_rx.next().await {
            let data = data.join("");
            if let Err(error) = terminal_api::write::write(terminal, data).await {
                error!("Failed to write to the terminal: {error}");
                return;
            }
        }
    }
    .instrument(debug_span!("WriteLoop"))
    .await
}
