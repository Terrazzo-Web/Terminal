use nameth::NamedEnumValues as _;
use nameth::nameth;
use tonic::Code;
use tonic::Status;

use super::callback::LogsCallback;
use super::callback::LogsLocalError;
use super::response::HybridResponseStream;
use crate::backend::client_service::remote_fn_service::remote_fn_server;
use crate::backend::client_service::routing::DistributedCallback as _;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::logs::LogsRequest;

pub async fn logs_dispatch(request: LogsRequest) -> Result<HybridResponseStream, LogsError> {
    let server = remote_fn_server().ok();
    let client_address = request
        .address
        .as_ref()
        .map(|address| address.via.clone())
        .unwrap_or_default();
    LogsCallback::process(server.as_ref(), &client_address, request)
        .await
        .map_err(LogsError::Error)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum LogsError {
    #[error("[{n}] {0}", n = self.name())]
    Error(DistributedCallbackError<LogsLocalError, Status>),
}

impl From<LogsError> for Status {
    fn from(mut error: LogsError) -> Self {
        let code = match &mut error {
            LogsError::Error(DistributedCallbackError::RemoteError(error)) => {
                return std::mem::replace(error, Status::ok(""));
            }
            LogsError::Error(DistributedCallbackError::LocalError { .. })
            | LogsError::Error(DistributedCallbackError::ServerNotSet) => Code::Internal,
            LogsError::Error(DistributedCallbackError::RemoteClientNotFound { .. }) => {
                Code::NotFound
            }
        };
        Status::new(code, error.to_string())
    }
}
