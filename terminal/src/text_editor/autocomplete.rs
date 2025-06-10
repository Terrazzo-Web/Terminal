use std::sync::Arc;

use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

use super::path_selector::PathSelector;
use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remote_server_fn::RemoteServerFn;

mod remote;
mod service;
pub mod ui;

static AUTOCOMPLETE_PATH_SERVER_FN: RemoteServerFn = RemoteServerFn {
    name: AUTOCOMPLETE_PATH,
    callback: remote::autocomplete_path,
};

#[server]
#[nameth]
async fn autocomplete_path(
    address: ClientAddress,
    kind: PathSelector,
    prefix: Arc<str>,
    input: String,
) -> Result<Vec<String>, ServerFnError> {
    let args = remote::AutoCompletePathArg {
        address,
        kind,
        prefix,
        input,
    };
    return Ok(AUTOCOMPLETE_PATH_SERVER_FN.call(args).await?);
}
