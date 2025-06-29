#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::prelude::*;

use super::fsio;
use super::notify::ui::NotifyService;
use super::side::SideViewList;
use super::synchronized_state::SynchronizedState;
use crate::frontend::remotes::Remote;

// TODO: rename to TextEditorManager
pub(super) struct TextEditor {
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

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("base_path", &self.base_path)
            .field("file_path", &self.file_path)
            .field("data", &self.data)
            .finish()
    }
}
