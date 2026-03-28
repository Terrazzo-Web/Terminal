use std::sync::Arc;
use std::sync::LazyLock;

use self::diagnostics::warn;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::engine::ClientLogEvent;
use super::engine::LogsEngine;
use crate::frontend::mousemove::MousemoveManager;
use crate::frontend::mousemove::Position;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

stylance::import_style!(style, "panel.scss");

#[html]
#[template(tag = div)]
pub fn panel() -> XElement {
    let logs_engine = LogsEngine::new();
    let logs = logs_engine.logs();
    let show_logs_panel = XSignal::new("show-logs-panel", false);
    tag(
        before_render = move |_| {
            let _ = &logs_engine;
        },
        show_resize_bar(show_logs_panel.clone()),
        show_logs(logs, show_logs_panel.clone()),
    )
}

#[html]
#[template(tag = div)]
fn show_logs(
    #[signal] logs: Arc<Vec<ClientLogEvent>>,
    #[signal] mut show_logs_panel: bool,
) -> XElement {
    if show_logs_panel {
        tag(
            class = style::logs_panel,
            after_render = after_logs_render,
            style::height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            style::max_height %= logs_panel_height(RESIZE_MANAGER.delta.clone()),
            ol(
                class = style::logs_list,
                logs.iter()
                    .map(|log| {
                        let level = &log.level;
                        let message = &log.message;
                        li(
                            key = log.id.to_string(),
                            class = style::log_item,
                            div(class = style::log_level, "{level}"),
                            div(class = style::log_message, "{message}"),
                        )
                    })
                    .collect::<Vec<_>>()..,
            ),
            mouseleave = move |_: MouseEvent| show_logs_panel_mut.set(false),
        )
    } else {
        tag(style::display = "none")
    }
}

fn after_logs_render(element: &web_sys::Element) {
    const DEFAULT_LINE_HEIGHT: i32 = 20;
    let element: &web_sys::HtmlElement = element.dyn_ref().or_throw("logs panel");
    let scroll_top = element.scroll_top();
    let client_height = element.client_height();
    let scroll_height = element.scroll_height();
    let gap = scroll_height - (scroll_top + client_height);

    // Keep live-tail behavior only when user is near bottom (1-2 lines). If user has scrolled up, preserve position.
    let li = element.query_selector("li.log-item").ok().flatten();
    let line_height = li
        .and_then(|li| {
            li.dyn_ref::<web_sys::HtmlElement>()
                .map(|li| li.client_height())
        })
        .unwrap_or_else(|| {
            warn!("Failed to get log item height, defaulting to {DEFAULT_LINE_HEIGHT}px");
            DEFAULT_LINE_HEIGHT
        });

    if gap <= line_height * 2 {
        element.set_scroll_top(element.scroll_height());
    }
}

#[template(wrap = true)]
fn logs_panel_height(#[signal] position: Option<Position>) -> XAttributeValue {
    position.map(|position| format!("max(3rem, calc(14rem - {}px))", position.y))
}

#[html]
fn show_resize_bar(show_logs_panel: XSignal<bool>) -> XElement {
    div(
        class = style::logs_resize_bar,
        mouseover = move |_: MouseEvent| show_logs_panel.set(true),
        mousedown = RESIZE_MANAGER.mousedown(),
        dblclick = |_| RESIZE_MANAGER.delta.set(None),
        div(div()),
    )
}

static RESIZE_MANAGER: LazyLock<MousemoveManager> = LazyLock::new(MousemoveManager::new);
