use std::sync::Arc;

use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

use super::path_selector::PathSelector;
use crate::api::client_address::ClientAddress;

mod remote;
mod service;
pub mod ui;

#[server]
#[nameth]
async fn autocomplete_path(
    address: ClientAddress,
    kind: PathSelector,
    prefix: Arc<str>,
    input: String,
) -> Result<Vec<String>, ServerFnError> {
    let request = remote::AutoCompletePathRequest {
        kind,
        prefix,
        input,
    };
    return Ok(remote::AUTOCOMPLETE_PATH_SERVER_FN
        .call(address, request)
        .await?);
}
