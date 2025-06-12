#![cfg(feature = "client")]

use std::sync::Arc;

use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use super::editor::EditorState;
use super::editor::editor;
use super::fsio::load_file;
use super::state;
use super::style;
use super::synchronized_state::SynchronizedState;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::text_editor::remotes::show_remote;
use crate::text_editor::synchronized_state::show_synchronized_state;

/// The UI for the text editor app.
#[html]
#[template]
pub fn text_editor() -> XElement {
    let remote = XSignal::new("remote", None);
    div(
        style = "height: 100%;",
        text_editor_impl(remote.clone(), remote),
    )
}

#[html]
#[template(tag = div)]
fn text_editor_impl(#[signal] remote: Remote, remote_signal: XSignal<Remote>) -> XElement {
    let text_editor = Arc::new(TextEditor {
        remote,
        base_path: XSignal::new("base-path", Arc::default()),
        file_path: XSignal::new("file-path", Arc::default()),
        editor_state: XSignal::new("editor-state", None),
        synchronized_state: XSignal::new("synchronized-state", SynchronizedState::Sync),
    });

    text_editor.restore_paths();
    let base_path_subscriber =
        text_editor.save_on_change(text_editor.base_path.clone(), state::base_path::set);
    let file_path_subscriber =
        text_editor.save_on_change(text_editor.file_path.clone(), state::file_path::set);
    let file_async_view = text_editor.make_file_async_view();

    div(
        key = "text-editor",
        class = style::text_editor,
        div(
            class = style::header,
            menu(),
            text_editor.base_path_selector(),
            text_editor.file_path_selector(),
            show_synchronized_state(text_editor.synchronized_state.clone()),
            show_remote(remote_signal),
        ),
        editor(text_editor.editor_state.clone(), text_editor.clone()),
        after_render = move |_| {
            let _moved = &base_path_subscriber;
            let _moved = &file_path_subscriber;
            let _moved = &file_async_view;
        },
    )
}

impl TextEditor {
    /// Restores the paths
    #[autoclone]
    #[nameth]
    fn restore_paths(&self) {
        let Self {
            remote,
            base_path,
            file_path,
            ..
        } = self;
        spawn_local(async move {
            autoclone!(remote, base_path, file_path);
            let remote: Remote = remote.clone();
            let (get_base_path, get_file_path) = futures::future::join(
                state::base_path::get(remote.clone()),
                state::file_path::get(remote),
            )
            .await;
            if get_base_path.is_err() && get_file_path.is_err() {
                return;
            }
            let batch = Batch::use_batch(Self::RESTORE_PATHS);
            if let Ok(p) = get_base_path {
                base_path.set(p);
            }
            if let Ok(p) = get_file_path {
                file_path.set(p);
            }
            drop(batch);
        });
    }

    #[autoclone]
    fn make_file_async_view(self: &Arc<Self>) -> Consumers {
        let this = self;
        this.file_path.add_subscriber(move |file_path| {
            autoclone!(this);
            this.editor_state.force(None);
            let task = async move {
                autoclone!(this);
                let base_path = this.base_path.get_value_untracked();
                let data = load_file(this.remote.clone(), base_path.clone(), file_path.clone())
                    .await
                    .unwrap_or_else(|error| Some(error.to_string().into()));

                if let Some(data) = data {
                    this.editor_state.force(EditorState {
                        base_path,
                        file_path,
                        data,
                    })
                }
            };
            spawn_local(task);
        })
    }

    #[autoclone]
    fn save_on_change(
        &self,
        path: XSignal<Arc<str>>,
        setter: impl AsyncFn(Remote, Arc<str>) -> Result<(), ServerFnError> + Copy + 'static,
    ) -> Consumers {
        let remote = self.remote.clone();
        path.add_subscriber(move |p| {
            spawn_local(async move {
                autoclone!(remote);
                let () = setter(remote, p)
                    .await
                    .unwrap_or_else(|error| warn!("Failed to save path: {error}"));
            })
        })
    }
}

pub(super) struct TextEditor {
    pub remote: Remote,
    pub base_path: XSignal<Arc<str>>,
    pub file_path: XSignal<Arc<str>>,
    pub editor_state: XSignal<Option<EditorState>>,
    pub synchronized_state: XSignal<SynchronizedState>,
}
