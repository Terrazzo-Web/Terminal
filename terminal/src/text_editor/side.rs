use std::collections::BTreeMap;
use std::sync::Arc;

use crate::text_editor::fsio::FileMetadata;

pub mod mutation;
pub mod ui;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", allow(dead_code))]
pub enum SideViewNode {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "D"))]
    Folder(Arc<SideViewList>),
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "F"))]
    File(Arc<FileMetadata>),
}

pub type SideViewList = BTreeMap<Arc<str>, Arc<SideViewNode>>;

impl std::fmt::Debug for SideViewNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Folder(children) => f.debug_tuple("Folder").field(children).finish(),
            Self::File(file) => f.debug_tuple("File").field(&file.name).finish(),
        }
    }
}
