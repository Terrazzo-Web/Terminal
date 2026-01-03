#![cfg(feature = "client")]

use std::ops::Not;
use std::sync::Arc;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce as _;
use terrazzo::widgets::element_capture::ElementCapture;
use wasm_bindgen_futures::spawn_local;
use web_sys::FocusEvent;
use web_sys::HtmlInputElement;
use web_sys::KeyboardEvent;

use super::state::EditorSearchState;
use crate::assets::icons;
use crate::frontend::timestamp::datetime::DateTime;
use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::manager::EditorState;
use crate::text_editor::manager::TextEditorManager;
use crate::text_editor::style;

impl TextEditorManager {
    #[autoclone]
    #[html]
    pub fn search_selector(self: &Ptr<Self>) -> XElement {
        let is_active = XSignal::new("is-search-active", false);
        let input: ElementCapture<HtmlInputElement> = ElementCapture::default();

        return div(
            class = style::path_selector,
            style::flex_basis %= move |t| {
                autoclone!(is_active);
                flex_basis(t, is_active.clone())
            },
            img(
                class = format!("{} {}", style::path_selector_icon, style::search_icon),
                src = icons::search(),
                click = move |_| {
                    autoclone!(is_active, input);
                    is_active.set(true);
                    let () = input.with(|i| i.focus()).or_throw("focus");
                },
            ),
            search_selector_input(self.clone(), input, self.path.base.clone(), is_active),
        );

        #[template]
        pub fn flex_basis(#[signal] is_active: bool) -> XAttributeValue {
            is_active.not().then_some("0")
        }
    }
}

#[autoclone]
#[html]
#[template(tag = div)]
fn search_selector_input(
    manager: Ptr<TextEditorManager>,
    input: ElementCapture<HtmlInputElement>,
    #[signal] base: Arc<str>,
    #[signal] mut is_active: bool,
) -> XElement {
    if !is_active {
        return tag(style::display = "none", style::visibility = "hidden");
    }
    let do_search = Ptr::new(do_search(manager.clone(), base, input.clone()));
    let editor_state = manager.editor_state.clone();
    tag(
        class = style::path_selector_widget,
        key = "search",
        input(
            before_render = input.capture(),
            r#type = "text",
            class = style::path_selector_field,
            keydown = move |event: KeyboardEvent| {
                autoclone!(editor_state, is_active_mut, input, do_search);
                if event.key() == "Escape" {
                    event.prevent_default();
                    close_search(&editor_state, &is_active_mut);
                    let () = input.with(|i| i.blur()).or_throw("blur");
                    return;
                }
                do_search()
            },
            blur = move |_: FocusEvent| {
                autoclone!(editor_state);
                close_search(&editor_state, &is_active_mut);
            },
            focus = move |_: FocusEvent| start_search(&editor_state, &do_search),
        ),
    )
}

fn start_search(editor_state: &XSignal<EditorState>, do_search: &Ptr<impl Fn()>) {
    editor_state.update(|editor_state| {
        if let EditorState::Search { .. } = editor_state {
            return None;
        }
        Some(EditorState::Search(EditorSearchState {
            prev: Box::new(editor_state.clone()),
            results: Default::default(),
        }))
    });
    do_search()
}

fn close_search(editor_state: &XSignal<EditorState>, is_active_mut: &MutableSignal<bool>) {
    let batch = Batch::use_batch("close-search");
    editor_state.update(|editor_state| {
        let EditorState::Search(EditorSearchState { prev, .. }) = editor_state else {
            return None;
        };
        Some(prev.as_ref().clone())
    });
    is_active_mut.set(false);
    drop(batch);
}

fn do_search(
    manager: Ptr<TextEditorManager>,
    base: Arc<str>,
    input: ElementCapture<HtmlInputElement>,
) -> impl Fn() {
    let callback = Duration::from_millis(250)
        .async_debounce(move |()| do_search_impl(manager.clone(), base.clone(), input.clone()));
    move || spawn_local(callback(()))
}

async fn do_search_impl(
    manager: Ptr<TextEditorManager>,
    base: Arc<str>,
    input: ElementCapture<HtmlInputElement>,
) {
    let results = run_query(base, input).await;
    manager.editor_state.update_mut(move |editor_state| {
        let EditorState::Search(search_state) = editor_state else {
            return std::mem::take(editor_state);
        };
        search_state.results = results.into();
        std::mem::take(editor_state)
    });
}

async fn run_query(base: Arc<str>, input: ElementCapture<HtmlInputElement>) -> Vec<FileMetadata> {
    let query = input.with(|i| i.value());
    vec![
        FileMetadata {
            name: format!("{base}/{query}-1").into(),
            modified: Some(DateTime::now().utc()),
            ..Default::default()
        },
        FileMetadata {
            name: format!("{base}/{query}-2").into(),
            modified: Some(DateTime::now().utc()),
            ..Default::default()
        },
        FileMetadata {
            name: format!("{base}/{query}-3").into(),
            created: Some(DateTime::now().utc()),
            ..Default::default()
        },
    ]
}
