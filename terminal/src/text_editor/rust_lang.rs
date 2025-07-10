use std::sync::Arc;

use nameth::nameth;
use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use self::synthetic::SyntheticDiagnostic;
use crate::api::client_address::ClientAddress;

mod messages;
mod remote;
pub mod service;
pub mod synthetic;

#[server(protocol = Http<Json, Json>)]
#[nameth]
async fn cargo_check(
    remote: Option<ClientAddress>,
    base_path: Arc<str>,
    features: Vec<String>,
) -> Result<Vec<SyntheticDiagnostic>, ServerFnError> {
    use scopeguard::defer;
    use tracing::Instrument as _;
    use tracing::debug;
    use tracing::debug_span;
    async move {
        debug!("Start");
        defer!(debug!("End"));
        let request = remote::CargoCheckRequest {
            base_path,
            features,
        };
        return Ok(remote::CARGO_CHECK_REMOTE_FN
            .call(remote.unwrap_or_default(), request)
            .await?);
    }
    .instrument(debug_span!("CargoCheck"))
    .await
}
