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
    File {
        metadata: Arc<FileMetadata>,
        #[serde(skip)]
        notify_registration: opqaue::OpaqueNotifyRegistration,
    },
}

pub mod opqaue {
    use std::sync::Arc;

    use crate::text_editor::notify::ui::NotifyRegistration;

    #[derive(Clone, Default)]
    pub struct OpaqueNotifyRegistration(Option<Arc<NotifyRegistration>>);

    impl From<Arc<NotifyRegistration>> for OpaqueNotifyRegistration {
        fn from(value: Arc<NotifyRegistration>) -> Self {
            Self(Some(value))
        }
    }

    impl OpaqueNotifyRegistration {
        pub fn is_set(&self) -> bool {
            self.0.is_some()
        }
    }

    unsafe impl Send for OpaqueNotifyRegistration {}
    unsafe impl Sync for OpaqueNotifyRegistration {}
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
                .field("has_notify_registration", &notify_registration.is_set())
                .finish(),
        }
    }
}
