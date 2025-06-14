#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::debug;
use wasm_bindgen::JsValue;

use super::code_mirror::CodeMirrorJs;
use super::synchronized_state::SynchronizedState;
use crate::text_editor::fsio::ui::store_file;
use crate::text_editor::ui::EditorState;
use crate::text_editor::ui::TextEditor;

#[autoclone]
#[html]
#[template(tag = div)]
pub fn editor(
    text_editor: Arc<TextEditor>,
    editor_state: EditorState,
    content: Arc<str>,
) -> XElement {
    let EditorState {
        base_path,
        file_path,
        ..
    } = editor_state;

    let on_change: Closure<dyn FnMut(JsValue)> = Closure::new(move |content: JsValue| {
        let Some(content) = content.as_string() else {
            debug!("Changed content is not a string");
            return;
        };
        let write = async move {
            autoclone!(base_path, file_path, text_editor);
            let pending = SynchronizedState::enqueue(text_editor.synchronized_state.clone());
            let () = store_file(
                text_editor.remote.clone(),
                base_path,
                file_path,
                content,
                pending,
            )
            .await;
        };
        wasm_bindgen_futures::spawn_local(write);
    });

    tag(after_render = move |element| {
        drop(CodeMirrorJs::new(
            element,
            content.as_ref().into(),
            &on_change,
        ))
    })
}
