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
use tracing::info;
use tracing::info_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::Empty;
use crate::backend::protos::terrazzo::gateway::client::TerminalAddress;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::close::CloseProcessError;
use crate::terminal_id::TerminalId;

pub fn close(
    server: &Server,
    client_address: &[impl AsRef<str>],
    terminal_id: TerminalId,
) -> impl Future<Output = Result<(), CloseError>> {
    async {
        info!("Start");
        defer!(info!("Done"));
        Ok(CloseCallback::process(server, client_address, terminal_id).await?)
    }
    .instrument(info_span!("Close"))
}

struct CloseCallback;

impl DistributedCallback for CloseCallback {
    type Request = TerminalId;
    type Response = ();
    type LocalError = CloseProcessError;
    type RemoteError = Status;

    async fn local(_: &Server, terminal_id: TerminalId) -> Result<(), CloseProcessError> {
        processes::close::close(&terminal_id)
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        terminal_id: TerminalId,
    ) -> Result<(), Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let Empty {} = client
            .close(TerminalAddress {
                terminal_id: terminal_id.to_string(),
                via: Some(ClientAddress::of(client_address)),
            })
            .await?
            .into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum CloseError {
    #[error("[{n}] {0}", n = self.name())]
    CloseError(#[from] DistributedCallbackError<CloseProcessError, Status>),
}

impl IsHttpError for CloseError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::CloseError(error) => error.status_code(),
        }
    }
}

impl From<CloseError> for Status {
    fn from(error: CloseError) -> Self {
        match error {
            CloseError::CloseError(error) => error.into(),
        }
    }
}

impl From<CloseProcessError> for Status {
    fn from(error: CloseProcessError) -> Self {
        match error {
            error @ CloseProcessError::TerminalNotFound { .. } => {
                Status::not_found(error.to_string())
            }
        }
    }
}
