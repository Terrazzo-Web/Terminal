#![cfg(feature = "client")]

use std::sync::Arc;

use nameth::NamedType as _;
use nameth::nameth;
use terrazzo::prelude::*;

use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::manager::EditorState;

#[derive(Clone)]
#[nameth]
pub struct EditorSearchState {
    pub(super) prev: Box<EditorState>,
    pub results: Arc<Vec<FileMetadata>>,
}

impl std::fmt::Debug for EditorSearchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(EditorSearchState::type_name()).finish()
    }
}

pub struct SearchState {
    pub query: XSignal<Arc<str>>,
}

impl SearchState {
    pub fn new() -> Ptr<Self> {
        Self {
            query: XSignal::new("search-query", Default::default()),
        }
        .into()
    }
}
