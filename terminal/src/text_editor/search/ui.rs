#![cfg(feature = "client")]

use std::sync::Arc;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::FocusEvent;
use web_sys::HtmlInputElement;
use web_sys::KeyboardEvent;

use crate::assets::icons;
use crate::frontend::element_capture::ElementCapture;
use crate::frontend::timestamp::datetime::DateTime;
use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::manager::EditorState;
use crate::text_editor::manager::TextEditorManager;
use crate::text_editor::style;

use super::state::EditorSearchState;

impl TextEditorManager {
    #[html]
    pub fn search_selector(self: &Ptr<Self>) -> XElement {
        div(
            class = style::path_selector,
            search_selector_input(self.clone(), self.path.base.clone()),
            img(class = style::path_selector_icon, src = icons::search()),
        )
    }
}

#[autoclone]
#[html]
#[template(tag = div)]
fn search_selector_input(manager: Ptr<TextEditorManager>, #[signal] base: Arc<str>) -> XElement {
    let input = ElementCapture::default();
    let do_search = Ptr::new(do_search(manager.clone(), base, input.clone()));
    let editor_state = manager.editor_state.clone();
    div(
        class = style::path_selector_widget,
        key = "search",
        input(
            before_render = input.capture(),
            r#type = "text",
            class = style::path_selector_field,
            keydown = move |event: KeyboardEvent| {
                autoclone!(editor_state);
                if event.key() == "Escape" {
                    editor_state.update(|old| {
                        let EditorState::Search(EditorSearchState { prev, .. }) = old else {
                            return None;
                        };
                        Some(prev.as_ref().clone())
                    });
                    return;
                }
                do_search()
            },
            focus = move |_: FocusEvent| {
                editor_state.update(|old| {
                    if let EditorState::Search { .. } = old {
                        return None;
                    }
                    Some(EditorState::Search(EditorSearchState {
                        prev: Box::new(old.clone()),
                        results: Default::default(),
                    }))
                })
            },
        ),
    )
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
    let query = input.get().value();
    vec![
        FileMetadata {
            name: format!("{base}/{query}-1").into(),
            created: Some(DateTime::now().utc()),
            ..Default::default()
        },
        FileMetadata {
            name: format!("{base}/{query}-2").into(),
            created: Some(DateTime::now().utc()),
            ..Default::default()
        },
        FileMetadata {
            name: format!("{base}/{query}-3").into(),
            created: Some(DateTime::now().utc()),
            ..Default::default()
        },
    ]
}
