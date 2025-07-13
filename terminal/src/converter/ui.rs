#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast as _;
use web_sys::HtmlTextAreaElement;

use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes::show_remote;

stylance::import_crate_style!(style, "src/converter/converter.scss");

/// The UI for the converter app.
#[html]
#[template]
pub fn converter() -> XElement {
    let remote_signal: XSignal<Remote> = XSignal::new("remote", Remote::default());
    let left = XSignal::new("left", String::default());
    let right = left.view("right", |left| {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&left) {
            if let Ok(json) = serde_json::to_string_pretty(&json) {
                return json;
            }
        }
        return "error".into();
    });
    div(
        class = style::outer,
        converter_impl(remote_signal, left, right),
    )
}

#[autoclone]
#[html]
#[template(tag = div)]
fn converter_impl(
    remote_signal: XSignal<Remote>,
    #[signal] mut left: String,
    #[signal] mut right: String,
) -> XElement {
    let element: Arc<OnceLock<HtmlTextAreaElement>> = Default::default();
    div(
        class = style::inner,
        key = "converter",
        div(class = style::header, menu(), show_remote(remote_signal)),
        div(
            class = style::body,
            textarea(
                "{left}",
                before_render = move |e| {
                    autoclone!(element);
                    let _ = &right_mut;
                    element
                        .set(e.dyn_into().or_throw("Element not a textarea"))
                        .or_throw("Element was already set");
                },
                input = move |_: web_sys::InputEvent| {
                    autoclone!(element);
                    let element = element.get().or_throw("Element was not set");
                    let value = element.value();
                    diagnostics::debug!("Value: {value}");
                    left_mut.set(value);
                },
            ),
            pre("{right}"),
        ),
    )
}
