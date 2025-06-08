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
use super::path_selector::ui::base_path_selector;
use super::path_selector::ui::file_path_selector;
use super::state;
use super::style;
use super::synchronized_state::SynchronizedState;
use super::synchronized_state::show_synchronized_state;
use crate::frontend::menu::menu;

/// The UI for the text editor app.
#[html]
#[template]
pub fn text_editor() -> XElement {
    let base_path = XSignal::new("base-path", Arc::default());
    let file_path = XSignal::new("file-path", Arc::default());
    let editor_state = XSignal::new("editor-state", None);
    let synchronized_state = XSignal::new("synchronized-state", SynchronizedState::Sync);

    restore_paths(&base_path, &file_path);
    let base_path_subscriber = save_on_change(base_path.clone(), state::base_path::set);
    let file_path_subscriber = save_on_change(file_path.clone(), state::file_path::set);
    let file_async_view = make_file_async_view(&base_path, &file_path, &editor_state);

    div(
        style = "height: 100%;",
        div(
            key = "text-editor",
            class = style::text_editor,
            div(
                class = style::header,
                menu(),
                base_path_selector(base_path.clone()),
                file_path_selector(base_path, file_path),
                show_synchronized_state(synchronized_state.clone()),
            ),
            editor(editor_state, synchronized_state),
            after_render = move |_| {
                let _moved = &base_path_subscriber;
                let _moved = &file_path_subscriber;
                let _moved = &file_async_view;
            },
        ),
    )
}

/// Restores the paths
#[autoclone]
#[nameth]
fn restore_paths(base_path: &XSignal<Arc<str>>, file_path: &XSignal<Arc<str>>) {
    spawn_local(async move {
        autoclone!(base_path, file_path);
        let (get_base_path, get_file_path) =
            futures::future::join(state::base_path::get(), state::file_path::get()).await;
        if get_base_path.is_err() && get_file_path.is_err() {
            return;
        }
        let batch = Batch::use_batch(RESTORE_PATHS);
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
fn make_file_async_view(
    base_path: &XSignal<Arc<str>>,
    file_path: &XSignal<Arc<str>>,
    editor_state: &XSignal<Option<EditorState>>,
) -> Consumers {
    file_path.add_subscriber(move |file_path| {
        autoclone!(base_path, editor_state);
        editor_state.force(None);
        let task = async move {
            autoclone!(base_path, file_path, editor_state);
            let base_path = base_path.get_value_untracked();
            let data = load_file(base_path.clone(), file_path.clone())
                .await
                .unwrap_or_else(|error| Some(error.to_string().into()));

            if let Some(data) = data {
                editor_state.force(EditorState {
                    base_path,
                    file_path,
                    data,
                })
            }
        };
        spawn_local(task);
    })
}

fn save_on_change(
    path: XSignal<Arc<str>>,
    setter: impl AsyncFn(Arc<str>) -> Result<(), ServerFnError> + Copy + 'static,
) -> Consumers {
    path.add_subscriber(move |p| {
        spawn_local(async move {
            let () = setter(p)
                .await
                .unwrap_or_else(|error| warn!("Failed to save path: {error}"));
        })
    })
}
