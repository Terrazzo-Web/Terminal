#![cfg(feature = "client")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::tabs::TabDescriptor;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::spawn_local;
use web_sys::ClipboardEvent;
use web_sys::DragEvent;
use web_sys::KeyboardEvent;
use web_sys::MouseEvent;

use self::diagnostics::warn;
use super::api::Conversion;
use super::api::Conversions;
use crate::assets::icons;
use crate::converter::api::Language;
use crate::frontend::utils::SleepError;
use crate::frontend::utils::sleep;

stylance::import_style!(style, "conversion_tabs.scss");

impl TabsDescriptor for Conversions {
    type State = ConversionsState;
    type TabDescriptor = Conversion;

    fn tab_descriptors(&self) -> &[Self::TabDescriptor] {
        &self.conversions
    }

    fn after_titles(&self, state: &Self::State) -> impl IntoIterator<Item = impl Into<XNode>> {
        Some(copy(state))
    }
}

#[html]
pub fn copy(state: &ConversionsState) -> XElement {
    let label: XSignal<Label> = XSignal::new("copy-label", Label::default());
    copy_impl(label, state.selected.clone())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Label {
    #[default]
    Ready,
    Copying,
    Copied,
    Failed,
}

impl Label {
    #[html]
    #[template(tag = div)]
    fn show(#[signal] this: Label) -> XElement {
        match this {
            Label::Ready => div(img(class = style::copy_icon, src = icons::copy()), "Copy"),
            Label::Copying => div("Copying"),
            Label::Copied => div(img(class = style::copy_icon, src = icons::done()), "Copied"),
            Label::Failed => div("Failed"),
        }
    }
}

#[autoclone]
#[html]
#[template(tag = div)]
fn copy_impl(label: XSignal<Label>, #[signal] selected: Option<Conversion>) -> XElement {
    let Some(selected) = selected else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    tag(
        Label::show(label.clone()),
        class = style::label,
        click = move |_ev: MouseEvent| {
            let window = web_sys::window().or_throw("window");
            let clipboard = window.navigator().clipboard();
            let promise = clipboard.write_text(&selected.content);
            let future = JsFuture::from(promise);
            spawn_local(async move {
                autoclone!(label);
                label.set(Label::Copying);
                match future.await {
                    Ok(ok) => {
                        label.set(Label::Copied);
                        diagnostics::info!("Copied into clipboard {ok:?}");
                    }
                    Err(err) => {
                        label.set(Label::Failed);
                        warn!("Failed to clipboard copy {err:?}");
                    }
                }
                spawn_local(async move {
                    let sleep: Result<(), SleepError> = sleep(Duration::from_millis(700)).await;
                    sleep.or_else_throw(|error| format!("Failed to sleep: {error:?}"));
                    label.set(Label::default())
                });
            });
        },
    )
}

#[derive(Clone)]
pub struct ConversionsState {
    selected: XSignal<Option<Conversion>>,
    selected_tabs: Arc<HashMap<Language, XSignal<bool>>>,
}

impl ConversionsState {
    #[autoclone]
    pub fn new(conversions: &Conversions, preferred_language: XSignal<Option<Language>>) -> Self {
        let current_preferred_language = preferred_language.get_value_untracked();
        let selected = XSignal::new(
            "conversion-selected",
            current_preferred_language
                .or_else(|| conversions.conversions.first().map(|c| c.language.clone()))
                .and_then(|current_preferred_language| {
                    conversions
                        .conversions
                        .iter()
                        .find(|conversion| conversion.language == current_preferred_language)
                        .cloned()
                }),
        );
        let selected_tabs = conversions
            .conversions
            .iter()
            .map(|conversion| {
                let this = conversion.clone();
                let language = this.language.clone();
                let is_selected = selected.derive(
                    format!("selected-{language}"),
                    move |conversion| {
                        autoclone!(language);
                        conversion
                            .as_ref()
                            .map(|c: &Conversion| c.language == language)
                            .unwrap_or(false)
                    },
                    move |_, selected| {
                        autoclone!(preferred_language);
                        selected.then(|| {
                            preferred_language.set(this.language.clone());
                            Some(this.clone())
                        })
                    },
                );
                (language, is_selected)
            })
            .collect::<HashMap<_, _>>()
            .into();
        Self {
            selected,
            selected_tabs,
        }
    }
}

impl TabsState for ConversionsState {
    type TabDescriptor = Conversion;
    fn move_tab(&self, _after_tab: Option<Self::TabDescriptor>, _moved_tab_key: String) {}
}

impl TabDescriptor for Conversion {
    type State = ConversionsState;

    fn key(&self) -> XString {
        self.language.name.clone().into()
    }

    #[html]
    fn title(&self, _state: &Self::State) -> impl Into<XNode> {
        let language = self.language.name.clone();
        terrazzo::widgets::link::link(
            |_click| {},
            move || [span(class = super::ui::style::title_span, "{language}")],
        )
    }

    #[html]
    fn item(&self, _state: &Self::State) -> impl Into<XNode> {
        let content = &self.content;
        pre(
            "{content}",
            contenteditable = "true",
            keydown = |ev: KeyboardEvent| {
                struct KeyEvent<'t> {
                    alt_key: bool,
                    shift_key: bool,
                    ctrl_key: bool,
                    meta_key: bool,
                    key: &'t str,
                }
                match (KeyEvent {
                    alt_key: ev.alt_key(),
                    shift_key: ev.shift_key(),
                    ctrl_key: ev.ctrl_key(),
                    meta_key: ev.meta_key(),
                    key: &ev.key(),
                }) {
                    KeyEvent {
                        alt_key: false,
                        shift_key: false,
                        ctrl_key: true,
                        meta_key: false,
                        key: "c" | "a",
                    }
                    | KeyEvent {
                        alt_key: false,
                        shift_key: false,
                        ctrl_key: false,
                        meta_key: true,
                        key: "c" | "a",
                    }
                    | KeyEvent {
                        key: "ArrowDown" | "ArrowUp" | "ArrowLeft" | "ArrowRight",
                        ..
                    }
                    | KeyEvent {
                        key: "Home" | "End" | "PageUp" | "PageDown" | "Tab",
                        ..
                    } => {}
                    KeyEvent { .. } => ev.prevent_default(),
                }
            },
            drop = |ev: DragEvent| ev.prevent_default(),
            cut = |ev: ClipboardEvent| ev.prevent_default(),
            paste = |ev: ClipboardEvent| ev.prevent_default(),
        )
    }

    fn selected(&self, state: &Self::State) -> XSignal<bool> {
        state.selected_tabs.get(&self.language).unwrap().clone()
    }
}
