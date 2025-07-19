#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce;
use terrazzo::widgets::tabs::TabsOptions;
use terrazzo::widgets::tabs::tabs;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

use self::diagnostics::warn;
use super::api::Conversions;
use crate::converter::api::Language;
use crate::converter::tabs::ConversionsState;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;

stylance::import_crate_style!(pub(super) style, "src/converter/converter.scss");

/// The UI for the converter app.
#[html]
#[template]
pub fn converter() -> XElement {
    let remote: XSignal<Remote> = XSignal::new("remote", Remote::default());
    let conversions = XSignal::new("conversions", Conversions::default());
    let preferred_language = XSignal::new("preferred-language", None);
    div(
        class = style::outer,
        converter_impl(remote, conversions, preferred_language),
    )
}

#[html]
#[template(tag = div)]
fn converter_impl(
    remote_signal: XSignal<Remote>,
    conversions: XSignal<Conversions>,
    preferred_language: XSignal<Option<Language>>,
) -> XElement {
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
            show_input(remote_signal, conversions.clone()),
            show_conversions(conversions, preferred_language),
        ),
    )
}

#[autoclone]
#[html]
#[template(tag = textarea)]
fn show_input(#[signal] remote: Remote, conversions: XSignal<Conversions>) -> XElement {
    let element: Arc<OnceLock<HtmlTextAreaElement>> = Default::default();
    tag(
        before_render = move |e| {
            autoclone!(element);
            element
                .set(e.dyn_into().or_throw("Element not a textarea"))
                .or_throw("Element was already set");
        },
        input = move |_: web_sys::InputEvent| {
            autoclone!(remote, element, conversions);
            let element = element.get().or_throw("Element was not set");
            get_conversions(remote.clone(), element.value(), conversions.clone());
        },
    )
}

#[html]
#[template(tag = div)]
fn show_conversions(
    #[signal] conversions: Conversions,
    preferred_language: XSignal<Option<Language>>,
) -> XElement {
    let state = ConversionsState::new(&conversions, preferred_language);
    div(
        class = style::conversions,
        tabs(
            conversions,
            state,
            Ptr::new(TabsOptions {
                tabs_class: Some(style::tabs.into()),
                titles_class: Some(style::titles.into()),
                title_class: Some(style::title.into()),
                items_class: Some(style::items.into()),
                item_class: Some(style::item.into()),
                selected_class: Some(style::selected.into()),
                ..TabsOptions::default()
            }),
        ),
    )
}

fn get_conversions(remote: Remote, content: String, conversions: XSignal<Conversions>) {
    let debounced = get_conversions_debounced();
    debounced(GetConversionsUiRequest {
        remote,
        content,
        conversions,
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
        conversions: conversions_mut,
    }: GetConversionsUiRequest,
) {
    spawn_local(async move {
        let conversions = super::api::get_conversions(remote, content).await;
        match conversions {
            Ok(conversions) => conversions_mut.force(conversions),
            Err(error) => {
                warn!("Failed to get conversions: {error}");
                conversions_mut.force(Conversions::default())
            }
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
    conversions: XSignal<Conversions>,
}

struct DebouncedGetConversions(Box<dyn Fn(GetConversionsUiRequest)>);
unsafe impl Send for DebouncedGetConversions {}
unsafe impl Sync for DebouncedGetConversions {}
