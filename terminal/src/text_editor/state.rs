use std::sync::Arc;

use crate::state::make_state;

make_state!(base_path, Arc<str>);
make_state!(file_path, Arc<str>);

pub enum EditorContent {
    TextFile(String),
    Folder(Vec<FileMetadata>),
}

pub struct FileMetadata {
    pub name: String,
    pub size: u64,
    pub modified: usize,
}
