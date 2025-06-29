#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;

use terrazzo::prelude::*;

use super::fsio;
use super::notify::ui::NotifyService;
use super::side::SideViewList;
use super::synchronized_state::SynchronizedState;
use crate::frontend::remotes::Remote;
use crate::text_editor::side;

pub(super) struct TextEditorManager {
    pub remote: Remote,
    pub base_path: XSignal<Arc<str>>,
    pub file_path: XSignal<Arc<str>>,
    pub force_edit_path: XSignal<bool>,
    pub editor_state: XSignal<Option<EditorState>>,
    pub synchronized_state: XSignal<SynchronizedState>,
    pub side_view: XSignal<Arc<SideViewList>>,
    pub notify_service: Arc<NotifyService>,
}

#[derive(Clone)]
pub(super) struct EditorState {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
    pub data: Arc<fsio::File>,
}

impl TextEditorManager {
    pub fn watch_file(&self) {
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

    pub fn unwatch_file(&self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        self.side_view.update(|side_view| {
            self.notify_service.unwatch(
                &self.base_path.get_value_untracked(),
                &path.to_string_lossy(),
            );
            let path_vec: Vec<Arc<str>> = path
                .iter()
                .map(|leg| leg.to_string_lossy().to_string().into())
                .collect();
            side::mutation::remove_file(side_view.clone(), &path_vec).ok()
        });
    }
}

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("base_path", &self.base_path)
            .field("file_path", &self.file_path)
            .field("data", &self.data)
            .finish()
    }
}
