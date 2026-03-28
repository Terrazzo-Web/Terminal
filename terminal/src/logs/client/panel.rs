use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::element_capture::ElementCapture;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlDivElement;

use self::diagnostics::warn;
use super::engine::ClientLogEvent;
use super::engine::LogsEngine;
use crate::frontend::mousemove::MousemoveManager;
use crate::frontend::mousemove::Position;
use crate::frontend::sleep::sleep;

stylance::import_style!(style, "panel.scss");

#[html]
#[template(tag = div)]
pub fn panel() -> XElement {
    let show_logs_panel = XSignal::new("show-logs-panel", false);
    tag(
        show_resize_bar(show_logs_panel.clone()),
        show_logs(show_logs_panel.clone()),
    )
}

#[html]
#[template(tag = div)]
fn show_logs(#[signal] show_logs_panel: bool) -> XElement {
    if show_logs_panel {
        let logs_engine = LogsEngine::new();
        let logs = logs_engine.logs();
        let panel = ElementCapture::<HtmlDivElement>::default();
        let first_render = Cell::new(true).into();
        tag(
            class = style::logs_panel,
            before_render = panel.capture(),
            after_render = move |_| {
                let _ = &logs_engine;
            },
            style::height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            style::max_height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            show_logs_list(panel.clone(), first_render, logs),
        )
    } else {
        tag(style::display = "none")
    }
}

#[html]
#[template(tag = ol)]
fn show_logs_list(
    panel: ElementCapture<HtmlDivElement>,
    first_render: Ptr<Cell<bool>>,
    #[signal] logs: Arc<VecDeque<ClientLogEvent>>,
) -> XElement {
    tag(
        class = style::logs_list,
        after_render = move |_| after_logs_render(&first_render, logs.is_empty(), panel.clone()),
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
    panel: ElementCapture<HtmlDivElement>,
) {
    let panel = panel.get();
    if first_render.replace(logs_is_empty) {
        spawn_local(async move {
            let () = sleep(Duration::from_millis(0))
                .await
                .expect("Failed to sleep");
            let client_height = panel.client_height();
            let scroll_height = panel.scroll_height();
            panel.set_scroll_top(scroll_height - client_height);
        });
        return;
    }

    const DEFAULT_LINE_HEIGHT: i32 = 20;
    let scroll_top = panel.scroll_top();
    let client_height = panel.client_height();
    let scroll_height = panel.scroll_height();

    let gap = scroll_height - client_height - scroll_top;

    // Keep live-tail behavior only when user is near bottom (1-2 lines). If user has scrolled up, preserve position.
    let li = panel.query_selector("ol > li").ok().flatten();
    let line_height = li.map(|li| li.client_height()).unwrap_or_else(|| {
        warn!("Failed to get log item height, defaulting to {DEFAULT_LINE_HEIGHT}px");
        DEFAULT_LINE_HEIGHT
    });

    if gap <= line_height * 2 {
        panel.set_scroll_top(scroll_height - client_height);
    }
}

#[template(wrap = true)]
fn logs_panel_height(#[signal] position: Option<Position>) -> XAttributeValue {
    position.map(|position| format!("max(3rem, calc(14rem - {}px))", position.y))
}

#[autoclone]
#[html]
fn show_resize_bar(show_logs_panel: XSignal<bool>) -> XElement {
    div(
        class = style::logs_resize_bar,
        mouseover = move |_| {
            autoclone!(show_logs_panel);
            show_logs_panel.set(true)
        },
        mousedown = RESIZE_MANAGER.mousedown(),
        dblclick = move |_| show_logs_panel.set(false),
        div(div()),
    )
}

static RESIZE_MANAGER: LazyLock<MousemoveManager> = LazyLock::new(MousemoveManager::new);
