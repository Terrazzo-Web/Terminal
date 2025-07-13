#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use nameth::nameth;
use scopeguard::guard;
use server_fn::ServerFnError;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::debug;
use self::diagnostics::warn;
use super::editor::editor;
use super::file_path::FilePath;
use super::folder::folder;
use super::fsio;
use super::manager::EditorState;
use super::manager::TextEditorManager;
use super::notify::ui::NotifyService;
use super::side::SideViewList;
use super::side::ui::show_side_view;
use super::state;
use super::style;
use super::synchronized_state::SynchronizedState;
use super::synchronized_state::show_synchronized_state;
use crate::frontend::menu::menu;
use crate::frontend::remotes::show_remote;
use crate::frontend::remotes_list::Remote;

pub(super) const STORE_FILE_DEBOUNCE_DELAY: Duration = if cfg!(debug_assertions) {
    Duration::from_millis(1500)
} else {
    Duration::from_millis(500)
};

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
    let side_view: XSignal<Arc<SideViewList>> = XSignal::new("side-view", Default::default());
    let manager = Ptr::new(TextEditorManager {
        remote: remote.clone(),
        path: FilePath {
            base: XSignal::new("base-path", Arc::default()),
            file: XSignal::new("file-path", Arc::default()),
        },
        force_edit_path: XSignal::new("force-edit-path", false),
        editor_state: XSignal::new("editor-state", None),
        synchronized_state: XSignal::new("synchronized-state", SynchronizedState::Sync),
        side_view,
        notify_service: Ptr::new(NotifyService::new(remote)),
    });

    let consumers = Arc::default();
    manager.restore_paths(&consumers);

    div(
        key = "text-editor",
        class = style::text_editor,
        div(
            class = style::header,
            menu(),
            manager.base_path_selector(),
            manager.file_path_selector(),
            show_synchronized_state(manager.synchronized_state.clone()),
            show_remote(remote_signal),
        ),
        editor_body(manager.clone(), manager.editor_state.clone()),
        after_render = move |_| {
            let _moved = &consumers;
        },
    )
}

#[html]
fn editor_body(
    manager: Ptr<TextEditorManager>,
    editor_state: XSignal<Option<EditorState>>,
) -> XElement {
    div(
        class = super::style::body,
        show_side_view(manager.clone(), manager.side_view.clone()),
        editor_container(manager, editor_state),
    )
}

#[template(tag = div)]
#[html]
fn editor_container(
    manager: Ptr<TextEditorManager>,
    #[signal] editor_state: Option<EditorState>,
) -> XElement {
    let Some(editor_state) = editor_state else {
        return tag(class = super::style::editor_container);
    };

    let body = match &*editor_state.data {
        fsio::File::TextFile { content, .. } => {
            let content = content.clone();
            editor(manager.clone(), editor_state, content)
        }
        fsio::File::Folder(list) => {
            let list = list.clone();
            folder(manager.clone(), editor_state, list)
        }
        fsio::File::Error(error) => {
            warn!("Failed to load file: {error}");
            return tag(class = super::style::editor_container);
        }
    };

    tag(class = super::style::editor_container, body)
}

impl TextEditorManager {
    /// Restores the paths
    #[autoclone]
    #[nameth]
    fn restore_paths(self: &Ptr<Self>, consumers: &Arc<Mutex<Consumers>>) {
        let this = self;
        spawn_local(async move {
            autoclone!(this, consumers);
            let registrations = Consumers::default().append(this.make_file_async_view());
            let registrations = guard(registrations, |registrations| {
                *consumers.lock().unwrap() = registrations
                    .append(this.save_on_change(this.path.base.clone(), state::base_path::set))
                    .append(this.save_on_change(this.path.file.clone(), state::file_path::set))
                    .append(this.save_on_change(this.side_view.clone(), state::side_view::set))
                    .append(this.path.base.add_subscriber(move |_base_path| {
                        autoclone!(this);
                        this.side_view.force(Arc::default());
                        this.path.file.force(Arc::default());
                    }))
            });
            let remote: Remote = this.remote.clone();
            let (get_side_view, get_base_path, get_file_path) = futures::future::join3(
                state::side_view::get(remote.clone()),
                state::base_path::get(remote.clone()),
                state::file_path::get(remote.clone()),
            )
            .await;
            let batch = Batch::use_batch(Self::RESTORE_PATHS);
            if let Ok(p) = get_base_path {
                this.path.base.force(p);
            }
            if let Ok(p) = get_file_path {
                this.path.file.force(p);
            }
            if let Ok(side_view) = get_side_view {
                debug!("Setting side_view to {side_view:?}");
                this.side_view.force(side_view);
            }
            this.force_edit_path.set(
                this.path.base.get_value_untracked().is_empty()
                    || this.path.file.get_value_untracked().is_empty(),
            );

            drop(batch);
            drop(registrations);
        });
    }

    #[autoclone]
    fn make_file_async_view(self: &Ptr<Self>) -> Consumers {
        let this = self;
        this.path.file.add_subscriber(move |file_path| {
            autoclone!(this);
            let loading = SynchronizedState::enqueue(this.synchronized_state.clone());
            let task = async move {
                autoclone!(this);
                let path = FilePath {
                    base: this.path.base.get_value_untracked(),
                    file: file_path,
                };
                let data = fsio::ui::load_file(this.remote.clone(), path.clone())
                    .await
                    .unwrap_or_else(|error| Some(fsio::File::Error(error.to_string())))
                    .map(Arc::new);

                if let Some(fsio::File::TextFile { metadata, .. }) = data.as_deref() {
                    this.add_to_side_view(metadata, &path);
                }

                if let Some(data) = data {
                    this.editor_state.force(EditorState { path, data })
                } else {
                    this.editor_state.force(None);
                }
                drop(loading);
            };
            spawn_local(task);
        })
    }

    #[autoclone]
    fn save_on_change<T>(
        &self,
        path: XSignal<Arc<T>>,
        setter: impl AsyncFn(Remote, Arc<T>) -> Result<(), ServerFnError> + Copy + 'static,
    ) -> Consumers
    where
        T: ?Sized + 'static,
    {
        let remote = self.remote.clone();
        path.add_subscriber(move |p| {
            spawn_local(async move {
                autoclone!(remote);
                let () = setter(remote, p)
                    .await
                    .unwrap_or_else(|error| warn!("Failed to save: {error}"));
            })
        })
    }
}
