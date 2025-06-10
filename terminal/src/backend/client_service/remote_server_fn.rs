//! Forward server_fn calls to mesh clients.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::Weak;

use scopeguard::defer;
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
    pub callback: fn(String) -> String,
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
    type Response = ();
    type LocalError = ServerFnErrorImpl;
    type RemoteError = tonic::Status;

    async fn local(server: &Server, request: ServerFnRequest) -> Result<(), ServerFnErrorImpl> {
        let Some(remote_server_fns) = REMOTE_SERVER_FNS.get() else {
            panic!()
        };
        let Some(remote_server_fn) = remote_server_fns.get(request) else {
            todo!()
        };
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: ServerFnRequest,
    ) -> Result<(), tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let Empty {} = client.set_title(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ServerFnError {
    #[error("[{n}] {0}", n = self.name())]
    ServerFnError(#[from] DistributedCallbackError<ServerFnErrorImpl, tonic::Status>),
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
        match error {
            error @ ServerFnErrorImpl::TerminalNotFound { .. } => {
                Status::not_found(error.to_string())
            }
        }
    }
}
