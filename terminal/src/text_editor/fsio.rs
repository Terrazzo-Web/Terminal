use std::sync::Arc;
use std::time::Duration;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use super::file_path::FilePath;
use crate::api::client_address::ClientAddress;

pub mod canonical;
mod fsmetadata;
mod remote;
mod service;
pub mod ui;

#[nameth]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum File {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "D"))]
    TextFile {
        metadata: Arc<FileMetadata>,
        content: Arc<str>,
    },
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "F"))]
    Folder(Arc<Vec<FileMetadata>>),
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    Error(String),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "n"))]
    pub name: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "s"))]
    pub size: Option<u64>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    pub is_dir: bool,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "ct"))]
    pub created: Option<Duration>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "at"))]
    pub accessed: Option<Duration>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "mt"))]
    pub modified: Option<Duration>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "m"))]
    pub mode: Option<u32>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "u"))]
    pub user: Option<Arc<str>>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "g"))]
    pub group: Option<Arc<str>>,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(self.name());
        match self {
            Self::TextFile { content, .. } => f.field(&content.len()),
            Self::Folder(folder) => f.field(&folder.len()),
            Self::Error(error) => f.field(error),
        }
        .finish()
    }
}

#[server(protocol = Http<Json, Json>)]
#[nameth]
async fn load_file(
    remote: Option<ClientAddress>,
    path: FilePath<Arc<str>>,
) -> Result<Option<File>, ServerFnError> {
    Ok(remote::LOAD_FILE_REMOTE_FN
        .call(remote.unwrap_or_default(), remote::LoadFileRequest { path })
        .await?)
}

#[server(protocol = Http<Json, Json>)]
#[nameth]
async fn store_file_impl(
    remote: Option<ClientAddress>,
    path: FilePath<Arc<str>>,
    content: String,
) -> Result<(), ServerFnError> {
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(remote::STORE_FILE_REMOTE_FN
        .call(
            remote.unwrap_or_default(),
            remote::StoreFileRequest { path, content },
        )
        .await?)
}
