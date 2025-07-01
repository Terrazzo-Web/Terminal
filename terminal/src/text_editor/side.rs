use std::collections::BTreeMap;
use std::sync::Arc;

use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::notify::ui::NotifyRegistration;

pub mod mutation;
pub mod ui;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", allow(dead_code))]
pub enum SideViewNode {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "D"))]
    Folder(Arc<SideViewList>),
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "F"))]
    File {
        metadata: Arc<FileMetadata>,
        #[serde(skip)]
        notify_registration: Option<Arc<NotifyRegistration>>,
    },
}

pub type SideViewList = BTreeMap<Arc<str>, Arc<SideViewNode>>;

impl std::fmt::Debug for SideViewNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Folder(children) => f.debug_tuple("Folder").field(children).finish(),
            Self::File {
                metadata,
                notify_registration,
            } => f
                .debug_struct("File")
                .field("name", &metadata.name)
                .field("has_notify_registration", &notify_registration.is_some())
                .finish(),
        }
    }
}
