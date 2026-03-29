use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::element_capture::ElementCapture;
use terrazzo::widgets::sleep::sleep;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlDivElement;

use self::diagnostics::warn;
use super::engine::ClientLogEvent;
use super::engine::LogsEngine;
use crate::assets::icons;
use crate::frontend::mousemove::MousemoveManager;
use crate::frontend::mousemove::Position;
use crate::frontend::remotes::Remote;

stylance::import_style!(style, "panel.scss");

#[html]
#[template(tag = div)]
pub fn panel(remote: XSignal<Remote>) -> XElement {
    let show_logs_panel = XSignal::new("show-logs-panel", false);
    tag(
        resize_bar(show_logs_panel.clone()),
        logs_panel(show_logs_panel.clone(), remote),
    )
}

#[html]
#[template(tag = div)]
fn logs_panel(#[signal] show_logs_panel: bool, #[signal] remote: Remote) -> XElement {
    if show_logs_panel {
        let logs_engine = LogsEngine::new(remote);
        let logs = logs_engine.logs();
        let logs_panel = ElementCapture::<HtmlDivElement>::default();
        let first_render = Cell::new(true).into();
        tag(
            class = style::logs_panel,
            before_render = logs_panel.capture(),
            after_render = move |_| {
                let _ = &logs_engine;
            },
            style::height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            style::max_height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            logs_list(logs_panel.clone(), first_render, logs),
        )
    } else {
        tag(style::display = "none")
    }
}

#[html]
#[template(tag = ol)]
fn logs_list(
    logs_panel: ElementCapture<HtmlDivElement>,
    first_render: Ptr<Cell<bool>>,
    #[signal] logs: Arc<VecDeque<ClientLogEvent>>,
) -> XElement {
    tag(
        class = style::logs_list,
        after_render =
            move |_| after_logs_render(&first_render, logs.is_empty(), logs_panel.clone()),
        logs.iter().map(|log| {
            let level = &log.level;
            let message = &log.message;
            li(
                key = log.id.to_string(),
                class = style::log_item,
                div(class = style::log_level, "{level}"),
                div(class = style::log_message, "{message}"),
            )
        })..,
    )
}

fn after_logs_render(
    first_render: &Cell<bool>,
    logs_is_empty: bool,
    logs_panel: ElementCapture<HtmlDivElement>,
) {
    let logs_panel = logs_panel.get();
    if first_render.replace(logs_is_empty) {
        spawn_local(async move {
            let () = sleep(Duration::from_millis(0))
                .await
                .expect("Failed to sleep");
            let client_height = logs_panel.client_height();
            let scroll_height = logs_panel.scroll_height();
            logs_panel.set_scroll_top(scroll_height - client_height);
        });
        return;
    }

    const DEFAULT_LINE_HEIGHT: i32 = 20;
    let scroll_top = logs_panel.scroll_top();
    let client_height = logs_panel.client_height();
    let scroll_height = logs_panel.scroll_height();

    let gap = scroll_height - client_height - scroll_top;

    // Keep live-tail behavior only when user is near bottom (1-2 lines). If user has scrolled up, preserve position.
    let li = logs_panel.query_selector("ol > li").ok().flatten();
    let line_height = li.map(|li| li.client_height()).unwrap_or_else(|| {
        warn!("Failed to get log item height, defaulting to {DEFAULT_LINE_HEIGHT}px");
        DEFAULT_LINE_HEIGHT
    });

    if gap <= line_height * 2 {
        logs_panel.set_scroll_top(scroll_height - client_height);
    }
}

#[template(wrap = true)]
fn logs_panel_height(#[signal] position: Option<Position>) -> XAttributeValue {
    position.map(|position| format!("max(3rem, calc(14rem - {}px))", position.y))
}

#[html]
fn resize_bar(show_logs_panel: XSignal<bool>) -> XElement {
    let resize_bar_visibility = resize_bar_visibility(show_logs_panel.clone());
    div(
        class = style::resize_bar,
        mousedown = RESIZE_MANAGER.mousedown(),
        dblclick = |_| RESIZE_MANAGER.delta.set(None),
        div(
            img(
                class = style::resize_icon,
                class %= resize_icon_class(show_logs_panel.clone()),
                src %= resize_icon_src(show_logs_panel.clone()),
                alt = "Resize logs panel",
                click = move |_| show_logs_panel.update(|t| Some(!t)),
            ),
            div(class %= resize_bar_visibility),
        ),
    )
}

#[template(wrap = true)]
pub fn resize_icon_class(#[signal] mut show_logs_panel: bool) -> XAttributeValue {
    if show_logs_panel {
        style::resize_icon_show
    } else {
        style::resize_icon_hide
    }
}

#[template(wrap = true)]
pub fn resize_icon_src(#[signal] mut show_logs_panel: bool) -> XAttributeValue {
    if show_logs_panel {
        icons::chevron_bar_down()
    } else {
        icons::chevron_bar_up()
    }
}

#[template(wrap = true)]
pub fn resize_bar_visibility(#[signal] mut show_logs_panel: bool) -> XAttributeValue {
    (!show_logs_panel).then_some(style::resize_bar_hidden)
}

static RESIZE_MANAGER: LazyLock<MousemoveManager> = LazyLock::new(MousemoveManager::new);
