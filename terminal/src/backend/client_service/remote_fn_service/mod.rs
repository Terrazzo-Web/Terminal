//! Forward [server_fn] calls to mesh clients.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Weak;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tonic::Status;
use trz_gateway_server::server::Server;

use crate::backend::client_service::remote_fn_service::remote_fn::RegisteredRemoteFn;
use crate::backend::client_service::routing::DistributedCallbackError;

mod callback;
mod dispatch;
mod grpc;
pub mod remote_fn;
pub mod uplift;

/// Records the current [Server] instance.
///
/// This is necessary because remote functions are static.
static SERVER: OnceLock<Weak<Server>> = OnceLock::new();

/// The collection of remote functions, declared using the [::inventory] crate.
static REMOTE_FNS: OnceLock<HashMap<&'static str, RegisteredRemoteFn>> = OnceLock::new();

inventory::collect!(RegisteredRemoteFn);

/// Initialize the server and the list of remote functions.
pub fn setup(server: &Arc<Server>) {
    let mut map: HashMap<&'static str, RegisteredRemoteFn> = HashMap::new();
    for remote_server_fn in inventory::iter::<RegisteredRemoteFn> {
        let old = map.insert(remote_server_fn.name, *remote_server_fn);
        assert! { old.is_none(), "Duplicate RemoteFn: {}", old.unwrap().name };
    }
    let Ok(()) = REMOTE_FNS.set(map) else {
        panic!("REMOTE_SERVER_FNS was already set");
    };
    SERVER.set(Arc::downgrade(server)).unwrap();
}

pub fn remote_fn_server() -> Result<Arc<Server>, RemoteFnError> {
    let server = SERVER.get().ok_or(RemoteFnError::ServerNotSet)?;
    server.upgrade().ok_or(RemoteFnError::ServerWasDropped)
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
    Distributed(#[from] Box<DistributedCallbackError<RemoteFnError, Status>>),
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
    (
        $remote_fn:ident,
        $remote_fn_name:expr,
        $input:ty,
        $output:ty,
        $implem:expr
    ) => {
        pub static $remote_fn: remote_fn_service::remote_fn::RemoteFn<$input, $output> = {
            fn callback(
                server: &std::sync::Arc<trz_gateway_server::server::Server>,
                arg: &str,
            ) -> remote_fn_service::remote_fn::RemoteFnResult {
                let callback = remote_fn_service::uplift::uplift::<$input, _, $output, _>($implem);
                Box::pin(callback(server, arg))
            }

            static REMOTE_FN_REGISTRATION: remote_fn_service::remote_fn::RegisteredRemoteFn =
                remote_fn_service::remote_fn::RegisteredRemoteFn::new($remote_fn_name, callback);
            inventory::submit! { REMOTE_FN_REGISTRATION };
            remote_fn_service::remote_fn::RemoteFn::new(REMOTE_FN_REGISTRATION)
        };
    };
}

pub(crate) use declare_remote_fn;
