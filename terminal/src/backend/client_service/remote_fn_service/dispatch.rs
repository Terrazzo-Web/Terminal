use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use crate::backend::client_service::remote_fn_service::RemoteFnError;
use crate::backend::client_service::remote_fn_service::callback::DistributedFn;
use crate::backend::client_service::routing::DistributedCallback as _;
use crate::backend::protos::terrazzo::remotefn::RemoteFnRequest;

/// Calls a [RemoteFn] using the [DistributedCallback] framework.
pub fn remote_fn_dispatch(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: RemoteFnRequest,
) -> impl Future<Output = Result<String, RemoteFnError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        DistributedFn::process(server, client_address, request)
            .await
            .map_err(|error| RemoteFnError::Distributed(Box::new(error)))
    }
    .instrument(debug_span!("DistributedFn"))
}
