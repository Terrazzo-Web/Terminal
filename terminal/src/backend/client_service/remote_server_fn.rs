//! Forward server_fn calls to mesh clients.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::Weak;

use scopeguard::defer;
use tonic::Status;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use crate::backend::protos::terrazzo::gateway::client::Empty;
use crate::backend::protos::terrazzo::gateway::client::ServerFnRequest;
use crate::backend::protos::terrazzo::gateway::client::ServerFnResponse;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

static SERVER: Mutex<Weak<Server>> = Mutex::new(Weak::new());
static REMOTE_SERVER_FNS: OnceLock<HashMap<&'static str, RemoteServerFn>> = OnceLock::new();

inventory::collect!(RemoteServerFn);

pub fn set_server(server: &Arc<Server>) {
    *SERVER.lock().unwrap() = Arc::downgrade(server);
    let mut map: HashMap<&'static str, RemoteServerFn> = HashMap::new();
    for remote_server_fn in inventory::iter::<RemoteServerFn> {
        let old = map.insert(remote_server_fn.name, *remote_server_fn);
        assert!(
            old.is_none(),
            "Duplicate RemoteServerFn {}",
            old.unwrap().name
        );
    }
    REMOTE_SERVER_FNS
        .set(map)
        .expect("REMOTE_SERVER_FNS was already set");
}

#[derive(Clone, Copy)]
pub struct RemoteServerFn {
    pub name: &'static str,
    pub callback: fn(String) -> Result<String, Box<dyn std::error::Error>>,
}

pub(super) fn handle_call(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: ServerFnRequest,
) -> impl Future<Output = Result<ServerFnResponse, ServerFnError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(ServerFnCallback::process(server, client_address, request).await?)
    }
    .instrument(debug_span!("ServerFn"))
}

struct ServerFnCallback;

impl DistributedCallback for ServerFnCallback {
    type Request = ServerFnRequest;
    type Response = String;
    type LocalError = ServerFnErrorImpl;
    type RemoteError = tonic::Status;

    async fn local(server: &Server, request: ServerFnRequest) -> Result<String, ServerFnErrorImpl> {
        let Some(remote_server_fns) = REMOTE_SERVER_FNS.get() else {
            return Err(ServerFnErrorImpl::RemoteServerFnNotSet);
        };
        let Some(remote_server_fn) = remote_server_fns.get(&request.server_fn_name) else {
            return Err(ServerFnErrorImpl::RemoteServerFnNotFound);
        };
        let callback = &remote_server_fn.callback;
        return Ok(callback(request.json)?);
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
        request.address.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let result = client.call_server_fn(request).await?.into_inner();
        Ok(result.json)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ServerFnError {
    #[error("[{n}] {0}", n = self.name())]
    ServerFnError(#[from] DistributedCallbackError<ServerFnErrorImpl, tonic::Status>),
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ServerFnErrorImpl {
    #[error("[{n}] REMOTE_SERVER_FNS was not set", n = self.name())]
    RemoteServerFnNotSet,

    #[error("[{n}] REMOTE_SERVER_FNS was not found", n = self.name())]
    RemoteServerFnNotFound,

    #[error("[{n}] {0}", n = self.name())]
    ServerFn(Box<dyn std::error::Error>)
}

impl IsHttpError for ServerFnError {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        match self {
            Self::ServerFnError(error) => error.status_code(),
        }
    }
}

impl From<ServerFnError> for Status {
    fn from(error: ServerFnError) -> Self {
        match error {
            ServerFnError::ServerFnError(error) => error.into(),
        }
    }
}

impl From<ServerFnErrorImpl> for Status {
    fn from(error: ServerFnErrorImpl) -> Self {
        match &error {
            ServerFnErrorImpl::RemoteServerFnNotSet => Status::internal(error.to_string()),
            ServerFnErrorImpl::RemoteServerFnNotFound => Status::not_found(error.to_string()),
            ServerFnErrorImpl::ServerFn(error) => Status::(error.to_string()),
        }
    }
}
