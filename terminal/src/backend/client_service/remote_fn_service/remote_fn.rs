use std::pin::Pin;

use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remote_fn_service::RemoteFnError;
use crate::backend::client_service::remote_fn_service::dispatch::dispatch;
use crate::backend::client_service::remote_fn_service::remote_fn_server;
use crate::backend::protos::terrazzo::remotefn::RemoteFnRequest;

/// A struct that holds a remote server function.
///
/// They must be statically registered using [inventory::submit].
#[derive(Clone, Copy)]
pub struct RemoteFn {
    pub name: &'static str,
    pub callback: fn(server: &Server, &str) -> RemoteFnResult,
}

/// Shorthand for the result of remote functions.
pub type RemoteFnResult = Pin<Box<dyn Future<Output = Result<String, RemoteFnError>> + Send>>;

impl RemoteFn {
    /// Calls the remote function.
    ///
    /// The remote function will be called on the client indicated by `address`.
    ///
    /// Takes care of serializing the request and then deserializing the response.
    pub fn call<Req, Res>(
        &self,
        address: ClientAddress,
        request: Req,
    ) -> impl Future<Output = Result<Res, RemoteFnError>>
    where
        Req: serde::Serialize,
        Res: for<'de> serde::Deserialize<'de>,
    {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let server = remote_fn_server()?;

            let request =
                serde_json::to_string(&request).map_err(RemoteFnError::SerializeRequest)?;

            let response = dispatch(
                &server,
                &address,
                RemoteFnRequest {
                    address: Default::default(),
                    server_fn_name: self.name.to_string(),
                    json: request,
                },
            )
            .await?;

            return serde_json::from_str(&response)
                .map_err(|error| RemoteFnError::DeserializeResponse(error, response));
        }
        .instrument(debug_span!("RemoteFn"))
    }
}
