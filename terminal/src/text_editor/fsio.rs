use std::sync::Arc;

use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

mod remote;
mod service;
pub mod ui;

#[server]
#[nameth]
pub async fn load_file(
    address: Option<ClientAddress>,
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<Arc<str>>, ServerFnError> {
    Ok(remote::LOAD_FILE_REMOTE_FN
        .call(
            address.unwrap_or_default(),
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
    address: Option<ClientAddress>,
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), ServerFnError> {
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(remote::STORE_FILE_REMOTE_FN
        .call(
            address.unwrap_or_default(),
            remote::StoreFileRequest {
                base_path,
                file_path,
                content,
            },
        )
        .await?)
}
