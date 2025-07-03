//! Forward [server_fn] calls to mesh clients.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Weak;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use pin_project::pin_project;
use scopeguard::defer;
use serde::Serialize;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::api::client_address::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::RemoteFnRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

/// Records the current [Server] instance.
///
/// This is necessary because remote functions are static.
static SERVER: OnceLock<Weak<Server>> = OnceLock::new();

/// The collection of remote functions, declared using the [::inventory] crate.
static REMOTE_FNS: OnceLock<HashMap<&'static str, RemoteFn>> = OnceLock::new();

inventory::collect!(RemoteFn);

/// Initialize the server and the list of remote functions.
pub fn setup(server: &Arc<Server>) {
    let mut map: HashMap<&'static str, RemoteFn> = HashMap::new();
    for remote_server_fn in inventory::iter::<RemoteFn> {
        let old = map.insert(remote_server_fn.name, *remote_server_fn);
        assert! { old.is_none(), "Duplicate RemoteFn: {}", old.unwrap().name };
    }
    let Ok(()) = REMOTE_FNS.set(map) else {
        panic!("REMOTE_SERVER_FNS was already set");
    };
    SERVER.set(Arc::downgrade(server)).unwrap();
}

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

pub fn remote_fn_server() -> Result<Arc<Server>, RemoteFnError> {
    let server = SERVER.get().ok_or(RemoteFnError::ServerNotSet)?;
    server.upgrade().ok_or(RemoteFnError::ServerWasDropped)
}

/// Helper to uplift a remote function into a String -> String server_fn.
pub const fn uplift<Req, F, Res, E>(
    function: impl Fn(&Server, Req) -> F + 'static,
) -> impl Fn(&Server, &str) -> UpliftFuture<F>
where
    Req: for<'de> serde::Deserialize<'de>,
    F: Future<Output = Result<Res, E>> + 'static,
    Res: serde::Serialize,
    Status: From<E>,
{
    move |server, request| {
        let request = serde_json::from_str::<Req>(request)
            .map_err(|error| RemoteFnError::DeserializeRequest(error, request.into()));
        match request {
            Ok(request) => UpliftFuture::Future(function(server, request)),
            Err(error) => UpliftFuture::DeserializeRequest(error),
        }
    }
}

#[pin_project(project=UpliftFutureProj)]
pub enum UpliftFuture<F> {
    DeserializeRequest(RemoteFnError),
    Future(#[pin] F),
}

impl<F, Res, E> Future for UpliftFuture<F>
where
    F: Future<Output = Result<Res, E>>,
    Res: Serialize,
    Status: From<E>,
{
    type Output = Result<String, RemoteFnError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            UpliftFutureProj::DeserializeRequest(error) => {
                let error = std::mem::replace(error, RemoteFnError::RemoteFnsNotSet);
                Err(error)
            }
            UpliftFutureProj::Future(future) => match ready!(future.poll(cx)) {
                Ok(response) => {
                    serde_json::to_string(&response).map_err(RemoteFnError::SerializeResponse)
                }
                Err(error) => Err(RemoteFnError::ServerFn(error.into())),
            },
        }
        .into()
    }
}

/// Calls a [RemoteFn] using the [DistributedCallback] framework.
pub(super) fn dispatch(
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

struct DistributedFn;

impl DistributedCallback for DistributedFn {
    type Request = RemoteFnRequest;
    type Response = String;
    type LocalError = RemoteFnError;
    type RemoteError = tonic::Status;

    async fn local(server: &Server, request: RemoteFnRequest) -> Result<String, RemoteFnError> {
        let Some(remote_server_fns) = REMOTE_FNS.get() else {
            return Err(RemoteFnError::RemoteFnsNotSet);
        };
        let Some(remote_server_fn) = remote_server_fns.get(request.server_fn_name.as_str()) else {
            return Err(RemoteFnError::RemoteFnNotFound(request.server_fn_name));
        };
        let callback = &remote_server_fn.callback;
        return callback(server, &request.json).await;
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: RemoteFnRequest,
    ) -> Result<String, tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address = Some(ClientAddressProto::of(client_address));
        let result = client.call_server_fn(request).await?.into_inner();
        Ok(result.json)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RemoteFnError {
    #[error("[{n}] REMOTE_FNS was not set", n = self.name())]
    RemoteFnsNotSet,

    #[error("[{n}] The RemoteFn was not found: {0}", n = self.name())]
    RemoteFnNotFound(String),

    #[error("[{n}] The Server instance was not set", n = self.name())]
    ServerNotSet,

    #[error("[{n}] The Server instance was dropped", n = self.name())]
    ServerWasDropped,

    #[error("[{n}] {0}", n = self.name())]
    ServerFn(Status),

    #[error("[{n}] Failed to serialize request: {0}", n = self.name())]
    SerializeRequest(serde_json::Error),

    #[error("[{n}] Failed to deserialize request: {0}", n = self.name())]
    DeserializeRequest(serde_json::Error, String),

    #[error("[{n}] Failed to serialize response: {0}", n = self.name())]
    SerializeResponse(serde_json::Error),

    #[error("[{n}] Failed to deserialize response: {0}, json='{1}'", n = self.name())]
    DeserializeResponse(serde_json::Error, String),

    #[error("[{n}] {0}", n = self.name())]
    Distributed(#[from] Box<DistributedCallbackError<RemoteFnError, tonic::Status>>),
}

/// Convert Remote Server function errors into gRPC status.
mod remote_fn_errors_to_status {
    use tonic::Status;

    use super::RemoteFnError;
    use crate::backend::client_service::routing::DistributedCallbackError;

    impl From<RemoteFnError> for Status {
        fn from(error: RemoteFnError) -> Self {
            match error {
                RemoteFnError::Distributed(mut error) => std::mem::replace(
                    error.as_mut(),
                    DistributedCallbackError::LocalError(RemoteFnError::RemoteFnsNotSet),
                )
                .into(),
                RemoteFnError::RemoteFnsNotSet
                | RemoteFnError::ServerNotSet
                | RemoteFnError::ServerWasDropped => Status::internal(error.to_string()),
                RemoteFnError::RemoteFnNotFound { .. } => Status::not_found(error.to_string()),
                RemoteFnError::ServerFn(error) => error,
                RemoteFnError::SerializeRequest { .. }
                | RemoteFnError::DeserializeRequest { .. }
                | RemoteFnError::SerializeResponse { .. }
                | RemoteFnError::DeserializeResponse { .. } => {
                    Status::invalid_argument(error.to_string())
                }
            }
        }
    }
}

macro_rules! declare_remote_fn {
    ($remote_fn:ident, $remote_fn_name:expr, $implem:expr) => {
        pub static $remote_fn: remote_fn::RemoteFn = {
            fn callback(
                server: &trz_gateway_server::server::Server,
                arg: &str,
            ) -> remote_fn::RemoteFnResult {
                let callback = remote_fn::uplift($implem);
                Box::pin(callback(server, arg))
            }

            remote_fn::RemoteFn {
                name: $remote_fn_name,
                callback,
            }
        };

        inventory::submit! { $remote_fn }
    };
}

pub(crate) use declare_remote_fn;
