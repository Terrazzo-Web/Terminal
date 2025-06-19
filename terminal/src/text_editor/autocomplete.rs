use std::sync::Arc;

use nameth::nameth;
use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use super::path_selector::PathSelector;
use crate::api::client_address::ClientAddress;

mod remote;
mod service;
pub mod ui;

#[server(protocol = Http<Json, Json>)]
#[nameth]
async fn autocomplete_path(
    remote: Option<ClientAddress>,
    kind: PathSelector,
    prefix: Arc<str>,
    input: String,
) -> Result<Vec<AutocompleteItem>, ServerFnError> {
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
        return Ok(remote::AUTOCOMPLETE_PATH_REMOTE_FN
            .call(remote.unwrap_or_default(), request)
            .await?);
    }
    .instrument(debug_span!("Autocomplete"))
    .await
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AutocompleteItem {
    #[cfg_attr(not(debug_assertions), serde(rename = "p"))]
    pub path: String,
    #[cfg_attr(not(debug_assertions), serde(rename = "d"))]
    pub is_dir: bool,
}
