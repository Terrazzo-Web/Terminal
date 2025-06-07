#![cfg(feature = "client")]

use std::num::NonZero;
use std::num::NonZeroU32;
use std::sync::Arc;

use server_fn::ServerFnError;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use self::editor::editor;
use self::path_selector::base_path_selector;
use self::path_selector::file_path_selector;
use super::load_file;
use super::state;
use crate::frontend::menu::menu;
use crate::text_editor::ui::editor::EditorState;
use crate::text_editor::ui::sync_status::sync_status_icon;

mod autocomplete;
mod code_mirror;
mod editor;
mod path_selector;
mod sync_status;

stylance::import_crate_style!(style, "src/text_editor/text_editor.scss");

#[autoclone]
#[html]
#[template]
pub fn text_editor() -> XElement {
    let base_path = XSignal::new("base-path", Arc::default());
    let file_path = XSignal::new("file-path", Arc::default());
    let editor_state = XSignal::new("editor-state", None);
    let synchronized_state = XSignal::new("synchronized-state", SynchronizedState::Sync);

    spawn_local(async move {
        autoclone!(base_path, file_path);
        if let Ok(p) = state::base_path::get().await {
            base_path.set(p);
        }
        if let Ok(p) = state::file_path::get().await {
            file_path.set(p);
        }
    });

    let base_path_subscriber = make_subs("base_path", base_path.clone(), state::base_path::set);
    let file_path_subscriber = make_subs("file_path", file_path.clone(), state::file_path::set);

    let file_async_view = file_path.add_subscriber(move |file_path| {
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
    });
    div(
        style = "height: 100%;",
        div(
            key = "text-editor",
            class = style::text_editor,
            div(
                class = style::header,
                before_render = move |_| {
                    let _moved = &file_async_view;
                },
                menu(),
                base_path_selector(base_path.clone()),
                file_path_selector(base_path, file_path),
                sync_status_icon(synchronized_state.clone()),
            ),
            editor(editor_state, synchronized_state),
            after_render = move |_| {
                let _ = &base_path_subscriber;
                let _ = &file_path_subscriber;
            },
        ),
    )
}

fn make_subs(
    name: &'static str,
    path: XSignal<Arc<str>>,
    setter: impl AsyncFn(Arc<str>) -> Result<(), ServerFnError> + Copy + 'static,
) -> Consumers {
    path.add_subscriber(move |p| {
        spawn_local(async move {
            let () = setter(p)
                .await
                .unwrap_or_else(|error| warn!("Failed to set {name}: {error}"));
        })
    })
}

#[derive(Clone, Copy, Debug)]
enum SynchronizedState {
    Sync,
    Pending(NonZero<u32>),
}

impl SynchronizedState {
    fn enqueue(state: XSignal<SynchronizedState>) -> impl Drop {
        state.update(|state| {
            Some(match state {
                Self::Sync => Self::Pending(NonZeroU32::MIN),
                Self::Pending(c) => Self::Pending(c.saturating_add(1)),
            })
        });
        scopeguard::guard(state, |state| {
            state.update(|state| {
                Some(match state {
                    Self::Sync => {
                        warn!("Impossible state:");
                        Self::Sync
                    }
                    Self::Pending(c) => (c.get() - 1)
                        .try_into()
                        .map(Self::Pending)
                        .unwrap_or(SynchronizedState::Sync),
                })
            });
        })
    }
}
