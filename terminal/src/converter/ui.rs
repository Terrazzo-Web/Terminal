#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;

stylance::import_crate_style!(style, "src/converter/converter.scss");

/// The UI for the converter app.
#[html]
#[template]
pub fn converter() -> XElement {
    let remote: XSignal<Remote> = XSignal::new("remote", Remote::default());
    let left = XSignal::new("left", String::default());
    let right = XSignal::new("right", String::default());
    div(
        class = style::outer,
        converter_impl(remote.clone(), remote, left, right),
    )
}

#[autoclone]
#[html]
#[template(tag = div)]
fn converter_impl(
    remote_signal: XSignal<Remote>,
    #[signal] remote: Remote,
    #[signal] mut left: String,
    #[signal] mut right: String,
) -> XElement {
    let element: Arc<OnceLock<HtmlTextAreaElement>> = Default::default();
    div(
        class = style::inner,
        key = "converter",
        div(
            class = style::header,
            menu(),
            show_remote(remote_signal.clone()),
        ),
        div(
            class = style::body,
            textarea(
                "{left}",
                before_render = move |e| {
                    autoclone!(element);
                    element
                        .set(e.dyn_into().or_throw("Element not a textarea"))
                        .or_throw("Element was already set");
                },
                input = move |_: web_sys::InputEvent| {
                    autoclone!(remote, element, right_mut);
                    let element = element.get().or_throw("Element was not set");
                    get_conversions(remote.clone(), element.value(), right_mut.clone());
                },
            ),
            pre("{right}"),
        ),
    )
}

fn get_conversions(remote: Remote, content: String, signal: MutableSignal<String>) {
    let debounced = get_conversions_debounced();
    debounced(GetConversionsUiRequest {
        remote,
        content,
        signal,
    })
}

fn get_conversions_debounced() -> &'static dyn Fn(GetConversionsUiRequest) {
    use std::sync::OnceLock;
    static DEBOUNCED: OnceLock<DebouncedGetConversions> = OnceLock::new();
    let debounced = DEBOUNCED.get_or_init(|| {
        DebouncedGetConversions(Box::new(DEBOUNCE_DELAY.debounce(spawn_conversions_request)))
    });
    &*debounced.0
}

fn spawn_conversions_request(
    GetConversionsUiRequest {
        remote,
        content,
        signal,
    }: GetConversionsUiRequest,
) {
    spawn_local(async move {
        let conversions = super::api::get_conversions(remote, content)
            .await
            .map(|conversions| conversions.conversions);
        let conversions = conversions.as_deref().map(Vec::as_slice);
        match conversions {
            Ok([first, ..]) => signal.set(first.content.clone()),
            Ok([]) => signal.set("No conversion found"),
            Err(error) => signal.set(error.to_string()),
        }
    })
}

static DEBOUNCE_DELAY: Duration = if cfg!(debug_assertions) {
    Duration::from_millis(700)
} else {
    Duration::from_millis(200)
};

struct GetConversionsUiRequest {
    remote: Remote,
    content: String,
    signal: MutableSignal<String>,
}

struct DebouncedGetConversions(Box<dyn Fn(GetConversionsUiRequest)>);
unsafe impl Send for DebouncedGetConversions {}
unsafe impl Sync for DebouncedGetConversions {}
