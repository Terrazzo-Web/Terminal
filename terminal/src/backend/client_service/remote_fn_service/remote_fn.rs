use std::marker::PhantomData;
use std::pin::Pin;

use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remote_fn_service::RemoteFnError;
use crate::backend::client_service::remote_fn_service::dispatch::remote_fn_dispatch;
use crate::backend::client_service::remote_fn_service::remote_fn_server;
use crate::backend::protos::terrazzo::remotefn::RemoteFnRequest;

/// A struct that holds a remote server function.
///
/// They must be statically registered using [inventory::submit].
pub struct RemoteFn<I, O> {
    delegate: RegisteredRemoteFn,
    _phantom: PhantomData<(I, O)>,
}

impl<I, O> Clone for RemoteFn<I, O> {
    fn clone(&self) -> Self {
        Self {
            delegate: self.delegate,
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RegisteredRemoteFn {
    pub(super) name: &'static str,
    pub(super) callback: fn(server: &Server, &str) -> RemoteFnResult,
}

impl RegisteredRemoteFn {
    pub const fn new(
        name: &'static str,
        callback: fn(server: &Server, &str) -> RemoteFnResult,
    ) -> Self {
        Self { name, callback }
    }
}

/// Shorthand for the result of remote functions.
pub type RemoteFnResult = Pin<Box<dyn Future<Output = Result<String, RemoteFnError>> + Send>>;

impl<I, O> RemoteFn<I, O> {
    pub const fn new(delegate: RegisteredRemoteFn) -> Self {
        Self {
            delegate,
            _phantom: PhantomData,
        }
    }

    /// Calls the remote function.
    ///
    /// The remote function will be called on the client indicated by `address`.
    ///
    /// Takes care of serializing the request and then deserializing the response.
    pub fn call(
        &self,
        address: ClientAddress,
        request: I,
    ) -> impl Future<Output = Result<O, RemoteFnError>>
    where
        I: serde::Serialize,
        O: for<'de> serde::Deserialize<'de>,
    {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let server = remote_fn_server()?;

            let request =
                serde_json::to_string(&request).map_err(RemoteFnError::SerializeRequest)?;

            let response = remote_fn_dispatch(
                &server,
                &address,
                RemoteFnRequest {
                    address: Default::default(),
                    server_fn_name: self.delegate.name.to_string(),
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
