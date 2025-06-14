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
use crate::backend::protos::terrazzo::gateway::client::Empty;
use crate::backend::protos::terrazzo::gateway::client::ResizeRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::resize::ResizeError as ResizeErrorImpl;

pub fn resize(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: ResizeRequest,
) -> impl Future<Output = Result<(), ResizeError>> {
    let terminal_id = request
        .terminal
        .as_ref()
        .map(|t| t.terminal_id.as_str())
        .unwrap_or_default();
    let span = debug_span!("Resize", %terminal_id);
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(ResizeCallback::process(server, client_address, request).await?)
    }
    .instrument(span)
}

struct ResizeCallback;

impl DistributedCallback for ResizeCallback {
    type Request = ResizeRequest;
    type Response = ();
    type LocalError = ResizeErrorImpl;
    type RemoteError = Status;

    async fn local(_: &Server, request: ResizeRequest) -> Result<(), ResizeErrorImpl> {
        let terminal_id = request.terminal.unwrap_or_default().terminal_id.into();
        let size = request.size.unwrap_or_default();
        processes::resize::resize(&terminal_id, size.rows, size.cols, request.force).await
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: ResizeRequest,
    ) -> Result<(), Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.terminal.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let Empty {} = client.resize(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ResizeError {
    #[error("[{n}] {0}", n = self.name())]
    ResizeError(#[from] DistributedCallbackError<ResizeErrorImpl, Status>),
}

impl IsHttpError for ResizeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ResizeError(error) => error.status_code(),
        }
    }
}

impl From<ResizeError> for Status {
    fn from(error: ResizeError) -> Self {
        match error {
            ResizeError::ResizeError(error) => error.into(),
        }
    }
}

impl From<ResizeErrorImpl> for Status {
    fn from(error: ResizeErrorImpl) -> Self {
        match error {
            error @ ResizeErrorImpl::TerminalNotFound { .. } => {
                Status::not_found(error.to_string())
            }
            ResizeErrorImpl::Resize(error) => Status::internal(error.to_string()),
        }
    }
}
