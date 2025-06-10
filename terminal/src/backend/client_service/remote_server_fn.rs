//! Forward server_fn calls to mesh clients.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Weak;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::http::StatusCode;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ServerFnRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

/// Records the current [Server] instance.
///
/// This is necessary because remote server functions are static.
static SERVER: OnceLock<Weak<Server>> = OnceLock::new();

/// The collection of remote server functions, declared using the [::inventory] crate.
static REMOTE_SERVER_FNS: OnceLock<HashMap<&'static str, RemoteServerFn>> = OnceLock::new();

inventory::collect!(RemoteServerFn);

/// Initialize the server and the list of remote server functions.
pub fn setup(server: &Arc<Server>) {
    let mut map: HashMap<&'static str, RemoteServerFn> = HashMap::new();
    for remote_server_fn in inventory::iter::<RemoteServerFn> {
        let old = map.insert(remote_server_fn.name, *remote_server_fn);
        assert!(
            old.is_none(),
            "Duplicate RemoteServerFn {}",
            old.unwrap().name
        );
    }
    let Ok(()) = REMOTE_SERVER_FNS.set(map) else {
        panic!("REMOTE_SERVER_FNS was already set");
    };
    SERVER.set(Arc::downgrade(server)).unwrap();
}

/// A struct that holds a remote server function.
///
/// They must be statically registered using [inventory::submit].
#[derive(Clone, Copy)]
pub struct RemoteServerFn {
    pub name: &'static str,
    pub callback: fn(String) -> RemoteServerFnResult,
}

/// Shorthand for the result of remote server functions.
pub type RemoteServerFnResult =
    Pin<Box<dyn Future<Output = Result<String, RemoteServerFnError>> + Send>>;

impl RemoteServerFn {
    pub async fn call<Req, Res>(&self, request: Req) -> Result<Res, RemoteServerFnError>
    where
        Req: serde::Serialize,
        Res: for<'de> serde::Deserialize<'de>,
    {
        let request =
            serde_json::to_string(&request).map_err(RemoteServerFnError::SerializeRequest)?;
        let callback = self.callback;
        let response = callback(request).await?;
        return serde_json::from_str(&response).map_err(RemoteServerFnError::DeserializeResponse);
    }
}

/// Helper to uplift a remote server function into a String -> String server_fn.
pub async fn call<Req, F, Res, E>(
    remote_server_fn: impl Fn(&Server, Req) -> F,
    address: ClientAddress,
    server_fn_name: String,
    request: String,
) -> Result<String, RemoteServerFnError>
where
    Req: for<'de> serde::Deserialize<'de>,
    F: Future<Output = Result<Res, E>>,
    Res: serde::Serialize,
    Status: From<E>,
{
    let server = SERVER.get().ok_or(RemoteServerFnError::ServerNotSet)?;
    let server = server
        .upgrade()
        .ok_or(RemoteServerFnError::ServerWasDropped)?;

    let c = call_internal(
        &server,
        &address.via,
        ServerFnRequest {
            address: Default::default(),
            server_fn_name,
            json: request,
        },
    )
    .await;

    let request =
        serde_json::from_str::<Req>(&request).map_err(RemoteServerFnError::DeserializeRequest)?;
    let response = remote_server_fn(&server, request)
        .await
        .map_err(|error| RemoteServerFnError::ServerFn(error.into()))?;
    let response = serde_json::to_string(&response);
    return Ok(response.map_err(RemoteServerFnError::SerializeResponse)?);
}

pub fn call_internal(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: ServerFnRequest,
) -> impl Future<Output = Result<String, DistributedServerFnError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        let result = ServerFnCallback::process(server, client_address, request)
            .await
            .map_err(DistributedServerFnError::Distributed);
        Ok(result?)
    }
    .instrument(debug_span!("ServerFn"))
}

struct ServerFnCallback;

impl DistributedCallback for ServerFnCallback {
    type Request = ServerFnRequest;
    type Response = String;
    type LocalError = RemoteServerFnError;
    type RemoteError = tonic::Status;

    async fn local(
        _server: &Server,
        request: ServerFnRequest,
    ) -> Result<String, RemoteServerFnError> {
        let Some(remote_server_fns) = REMOTE_SERVER_FNS.get() else {
            return Err(RemoteServerFnError::RemoteServerFnNotSet);
        };
        let Some(remote_server_fn) = remote_server_fns.get(request.server_fn_name.as_str()) else {
            return Err(RemoteServerFnError::RemoteServerFnNotFound);
        };
        let callback = &remote_server_fn.callback;
        return callback(request.json).await;
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: ServerFnRequest,
    ) -> Result<String, tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address = Some(ClientAddress::of(client_address));
        let result = client.call_server_fn(request).await?.into_inner();
        Ok(result.json)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DistributedServerFnError {
    #[error("[{n}] {0}", n = self.name())]
    Distributed(#[from] DistributedCallbackError<RemoteServerFnError, tonic::Status>),
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RemoteServerFnError {
    #[error("[{n}] REMOTE_SERVER_FNS was not set", n = self.name())]
    RemoteServerFnNotSet,

    #[error("[{n}] REMOTE_SERVER_FNS was not found", n = self.name())]
    RemoteServerFnNotFound,

    #[error("[{n}] The Server instance was not set", n = self.name())]
    ServerNotSet,

    #[error("[{n}] The Server instance was dropped", n = self.name())]
    ServerWasDropped,

    #[error("[{n}] {0}", n = self.name())]
    ServerFn(Status),

    #[error("[{n}] Failed to serialize request: {0}", n = self.name())]
    SerializeRequest(serde_json::Error),

    #[error("[{n}] Failed to deserialize request: {0}", n = self.name())]
    DeserializeRequest(serde_json::Error),

    #[error("[{n}] Failed to serialize response: {0}", n = self.name())]
    SerializeResponse(serde_json::Error),

    #[error("[{n}] Failed to deserialize response: {0}", n = self.name())]
    DeserializeResponse(serde_json::Error),
}

/// Convert Remote Server function errors into gRPC status.
mod server_fn_errors_to_status {
    use tonic::Status;

    use super::DistributedServerFnError;
    use super::RemoteServerFnError;

    impl From<DistributedServerFnError> for Status {
        fn from(error: DistributedServerFnError) -> Self {
            match error {
                DistributedServerFnError::Distributed(error) => error.into(),
            }
        }
    }

    impl From<RemoteServerFnError> for Status {
        fn from(error: RemoteServerFnError) -> Self {
            match error {
                RemoteServerFnError::RemoteServerFnNotSet
                | RemoteServerFnError::ServerNotSet
                | RemoteServerFnError::ServerWasDropped => Status::internal(error.to_string()),
                RemoteServerFnError::RemoteServerFnNotFound => Status::not_found(error.to_string()),
                RemoteServerFnError::ServerFn(error) => error,
                RemoteServerFnError::SerializeRequest(error)
                | RemoteServerFnError::DeserializeRequest(error)
                | RemoteServerFnError::SerializeResponse(error)
                | RemoteServerFnError::DeserializeResponse(error) => {
                    Status::invalid_argument(error.to_string())
                }
            }
        }
    }
}

// TODO: should not need to implement IsHttpError
impl IsHttpError for RemoteServerFnError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::RemoteServerFnNotSet | Self::ServerNotSet | Self::ServerWasDropped => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::RemoteServerFnNotFound => StatusCode::NOT_FOUND,
            Self::ServerFn { .. } => StatusCode::BAD_REQUEST,
            Self::SerializeRequest { .. }
            | Self::DeserializeRequest { .. }
            | Self::SerializeResponse { .. }
            | Self::DeserializeResponse { .. } => StatusCode::PRECONDITION_FAILED,
        }
    }
}
