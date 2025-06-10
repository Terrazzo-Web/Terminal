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
    address: Option<ClientAddress>,
    kind: PathSelector,
    prefix: Arc<str>,
    input: String,
) -> Result<Vec<String>, ServerFnError> {
    use scopeguard::defer;
    use tracing::Instrument as _;
    use tracing::debug;
    use tracing::debug_span;
    async move {
        debug!("Start");
        defer!(debug!("End"));
        let request = remote::AutoCompletePathRequest {
            kind,
            prefix,
            input,
        };
        return Ok(remote::AUTOCOMPLETE_PATH_SERVER_FN
            .call(address.unwrap_or_default(), request)
            .await?);
    }
    .instrument(debug_span!("Autocomplete"))
    .await
}
