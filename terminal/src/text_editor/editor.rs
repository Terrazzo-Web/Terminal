#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::SeqCst;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::debug;
use wasm_bindgen::JsValue;

use super::code_mirror::CodeMirrorJs;
use super::synchronized_state::SynchronizedState;
use crate::text_editor::fsio::ui::store_file;

#[derive(Clone)]
pub struct EditorState {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
    pub data: Arc<str>,
}

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("base_path", &self.base_path)
            .field("file_path", &self.file_path)
            .field("data", &self.data.len())
            .finish()
    }
}

#[autoclone]
#[html]
#[template(tag = div)]
pub fn editor(
    #[signal] editor_state: Option<EditorState>,
    synchronized_state: XSignal<SynchronizedState>,
) -> XElement {
    static NEXT: AtomicI32 = AtomicI32::new(1);
    let key = format!("editor-{}", NEXT.fetch_add(1, SeqCst));

    let Some(EditorState {
        base_path,
        file_path,
        data,
    }) = editor_state
    else {
        return tag(class = super::style::body, div(key = key));
    };

    let on_change: Closure<dyn FnMut(JsValue)> = Closure::new(move |content: JsValue| {
        let Some(content) = content.as_string() else {
            debug!("Changed content is not a string");
            return;
        };
        let write = async move {
            autoclone!(base_path, file_path, synchronized_state);
            let pending = SynchronizedState::enqueue(synchronized_state);
            let () = store_file(base_path, file_path, content, pending).await;
        };
        wasm_bindgen_futures::spawn_local(write);
    });

    tag(
        class = super::style::body,
        div(
            key = key,
            after_render =
                move |element| drop(CodeMirrorJs::new(element, data.as_ref().into(), &on_change)),
        ),
    )
}
