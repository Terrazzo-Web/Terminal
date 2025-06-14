use std::sync::Arc;
use std::time::Duration;

use nameth::NamedEnumValues;
use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

mod fsmetadata;
mod remote;
mod service;
pub mod ui;

#[nameth]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum File {
    TextFile(Arc<str>),
    Folder(Arc<Vec<FileMetadata>>),
    Error(String),
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub name: Arc<str>,
    pub size: Option<u64>,
    pub is_dir: bool,
    pub created: Option<Duration>,
    pub accessed: Option<Duration>,
    pub modified: Option<Duration>,
    pub mode: Option<u32>,
    pub user: Option<Arc<str>>,
    pub group: Option<Arc<str>>,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(self.name());
        match self {
            Self::TextFile(text_file) => f.field(&text_file.len()),
            Self::Folder(folder) => f.field(&folder.len()),
            Self::Error(error) => f.field(error),
        }
        .finish()
    }
}

#[server]
#[nameth]
pub async fn load_file(
    remote: Option<ClientAddress>,
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<File>, ServerFnError> {
    Ok(remote::LOAD_FILE_REMOTE_FN
        .call(
            remote.unwrap_or_default(),
            remote::LoadFileRequest {
                base_path,
                file_path,
            },
        )
        .await?)
}

#[server]
#[nameth]
async fn store_file_impl(
    remote: Option<ClientAddress>,
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), ServerFnError> {
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(remote::STORE_FILE_REMOTE_FN
        .call(
            remote.unwrap_or_default(),
            remote::StoreFileRequest {
                base_path,
                file_path,
                content,
            },
        )
        .await?)
}
