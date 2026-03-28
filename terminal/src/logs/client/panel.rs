use std::sync::Arc;
use std::sync::LazyLock;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::engine::ClientLogEvent;
use super::engine::LogsEngine;
use crate::frontend::mousemove::MousemoveManager;
use crate::frontend::mousemove::Position;

stylance::import_style!(style, "panel.scss");

#[html]
#[template(tag = div)]
pub fn panel() -> XElement {
    let logs_engine = LogsEngine::new();
    let logs = logs_engine.logs();
    tag(
        before_render = move |_| {
            let _ = &logs_engine;
        },
        show_resize_bar(),
        show_logs(logs),
    )
}

#[html]
#[template(tag = div)]
fn show_logs(#[signal] logs: Arc<Vec<ClientLogEvent>>) -> XElement {
    tag(
        class = style::logs_panel,
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
    )
}

#[template(wrap = true)]
fn logs_panel_height(#[signal] position: Option<Position>) -> XAttributeValue {
    position.map(|position| format!("max(3rem, calc(14rem - {}px))", position.y))
}

#[html]
fn show_resize_bar() -> XElement {
    div(
        class = style::logs_resize_bar,
        mousedown = RESIZE_MANAGER.mousedown(),
        dblclick = |_| RESIZE_MANAGER.delta.set(None),
        div(div()),
    )
}

static RESIZE_MANAGER: LazyLock<MousemoveManager> = LazyLock::new(MousemoveManager::new);
