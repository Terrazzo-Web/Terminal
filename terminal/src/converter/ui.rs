#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce;
use terrazzo::widgets::tabs::TabsOptions;
use terrazzo::widgets::tabs::tabs;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

use self::diagnostics::warn;
use super::api::Conversions;
use super::content_state;
use crate::converter::api::Language;
use crate::converter::conversion_tabs::ConversionsState;
use crate::frontend::element_capture::ElementCapture;
use crate::frontend::menu::menu;
use crate::frontend::mousemove::MousemoveManager;
use crate::frontend::mousemove::Position;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;

stylance::import_style!(pub(super) style, "converter.scss");

/// The UI for the converter app.
#[html]
#[template]
pub fn converter(remote: XSignal<Remote>) -> XElement {
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
    remote: XSignal<Remote>,
    conversions: XSignal<Conversions>,
    preferred_language: XSignal<Option<Language>>,
) -> XElement {
    div(
        class = style::inner,
        key = "converter",
        div(class = style::header, menu(), show_remote(remote.clone())),
        div(
            class = style::body,
            show_input(remote, conversions.clone()),
            show_resize_bar(),
            show_conversions(conversions, preferred_language),
        ),
    )
}

#[autoclone]
#[html]
#[template(tag = textarea)]
fn show_input(#[signal] remote: Remote, conversions: XSignal<Conversions>) -> XElement {
    let element = ElementCapture::<HtmlTextAreaElement>::default();
    tag(
        style::flex %= |t| width(t, RESIZE_MANAGER.delta.clone()),
        before_render = element.capture(),
        input = move |_: web_sys::InputEvent| {
            autoclone!(remote, element, conversions);
            let value: Arc<str> = element.with(|e| e.value().into());
            spawn_local(async move {
                autoclone!(remote, value);
                let _set = content_state::set(remote.clone(), value.clone()).await;
            });
            get_conversions(remote.clone(), value, conversions.clone());
        },
        after_render = move |_| {
            autoclone!(remote, element, conversions);
            spawn_local(async move {
                autoclone!(remote, element, conversions);
                let Ok(content) = content_state::get(remote.clone()).await else {
                    warn!("Failed to load converter content");
                    return;
                };
                element.with(|e| e.set_value(&content));
                get_conversions(remote.clone(), content, conversions.clone());
            });
        },
    )
}

#[template]
fn width(#[signal] mut position: Option<Position>) -> XAttributeValue {
    position.map(|position| format!("0 0 calc(50% + {}px)", position.x))
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

fn get_conversions(remote: Remote, content: Arc<str>, conversions: XSignal<Conversions>) {
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
    content: Arc<str>,
    conversions: XSignal<Conversions>,
}

struct DebouncedGetConversions(Box<dyn Fn(GetConversionsUiRequest)>);
unsafe impl Send for DebouncedGetConversions {}
unsafe impl Sync for DebouncedGetConversions {}

#[html]
fn show_resize_bar() -> XElement {
    div(
        class = style::resize_bar,
        mousedown = RESIZE_MANAGER.mousedown(),
        dblclick = |_| RESIZE_MANAGER.delta.set(None),
        div(div()),
    )
}

static RESIZE_MANAGER: LazyLock<MousemoveManager> = LazyLock::new(MousemoveManager::new);
