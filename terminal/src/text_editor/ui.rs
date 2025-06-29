#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::SeqCst;

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
use super::folder::folder;
use super::fsio;
use super::manager::EditorState;
use super::manager::TextEditorManager;
use super::notify::ui::NotifyService;
use super::remotes::show_remote;
use super::side;
use super::side::SideViewList;
use super::side::ui::show_side_view;
use super::state;
use super::style;
use super::synchronized_state::SynchronizedState;
use super::synchronized_state::show_synchronized_state;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;

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
    let manager = Arc::new(TextEditorManager {
        remote: remote.clone(),
        base_path: XSignal::new("base-path", Arc::default()),
        file_path: XSignal::new("file-path", Arc::default()),
        force_edit_path: XSignal::new("force-edit-path", false),
        editor_state: XSignal::new("editor-state", None),
        synchronized_state: XSignal::new("synchronized-state", SynchronizedState::Sync),
        side_view,
        notify_service: Arc::new(NotifyService::new(remote)),
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
    manager: Arc<TextEditorManager>,
    editor_state: XSignal<Option<EditorState>>,
) -> XElement {
    div(
        class = super::style::body,
        show_side_view(manager.clone(), manager.side_view.clone()),
        editor_container(manager, editor_state),
    )
}

#[template(tag = div, key = {
    static NEXT: AtomicI32 = AtomicI32::new(1);
    format!("editor-{}", NEXT.fetch_add(1, SeqCst))
})]
#[html]
fn editor_container(
    manager: Arc<TextEditorManager>,
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
        fsio::File::Error(_error) => todo!(),
    };

    tag(class = super::style::editor_container, body)
}

impl TextEditorManager {
    /// Restores the paths
    #[autoclone]
    #[nameth]
    fn restore_paths(self: &Arc<Self>, consumers: &Arc<Mutex<Consumers>>) {
        let this = self;
        spawn_local(async move {
            autoclone!(this, consumers);
            let registrations = Consumers::default().append(this.make_file_async_view());
            let registrations = guard(registrations, |registrations| {
                *consumers.lock().unwrap() = registrations
                    .append(this.save_on_change(this.base_path.clone(), state::base_path::set))
                    .append(this.save_on_change(this.file_path.clone(), state::file_path::set))
                    .append(this.save_on_change(this.side_view.clone(), state::side_view::set))
                    .append(this.base_path.add_subscriber(move |_base_path| {
                        autoclone!(this);
                        this.side_view.force(Arc::default());
                    }))
                    .append(this.base_path.add_subscriber(move |_base_path| {
                        autoclone!(this);
                        this.file_path.force(Arc::default());
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
                this.base_path.set(p);
            }
            if let Ok(p) = get_file_path {
                this.file_path.set(p);
            }
            if let Ok(side_view) = get_side_view {
                debug!("Setting side_view to {side_view:?}");
                this.side_view.force(side_view);
            }
            this.force_edit_path.set(
                this.base_path.get_value_untracked().is_empty()
                    || this.file_path.get_value_untracked().is_empty(),
            );

            drop(batch);
            drop(registrations);
        });
    }

    #[autoclone]
    fn make_file_async_view(self: &Arc<Self>) -> Consumers {
        let this = self;
        this.file_path.add_subscriber(move |file_path| {
            autoclone!(this);
            let loading = SynchronizedState::enqueue(this.synchronized_state.clone());
            this.editor_state.force(None);
            let task = async move {
                autoclone!(this);
                let base_path = this.base_path.get_value_untracked();
                let data =
                    fsio::ui::load_file(this.remote.clone(), base_path.clone(), file_path.clone())
                        .await
                        .unwrap_or_else(|error| Some(fsio::File::Error(error.to_string())))
                        .map(Arc::new);

                if let Some(fsio::File::TextFile { metadata, .. }) = data.as_deref() {
                    let relative_path = Path::new(file_path.as_ref())
                        .iter()
                        .map(|leg| Arc::from(leg.to_string_lossy().to_string()))
                        .collect::<Vec<_>>();
                    this.notify_service.watch(&base_path, &file_path);
                    this.side_view.update(|tree| {
                        Some(side::mutation::add_file(
                            tree.clone(),
                            relative_path.as_slice(),
                            super::side::SideViewNode::File(metadata.clone()),
                        ))
                    });
                    this.force_edit_path.set(false);
                }

                if let Some(data) = data {
                    this.editor_state.force(EditorState {
                        base_path,
                        file_path,
                        data,
                    })
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
                    .unwrap_or_else(|error| warn!("Failed to save path: {error}"));
            })
        })
    }
}
