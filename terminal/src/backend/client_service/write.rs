use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tonic::Status;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::TerminalAddress;
use crate::backend::protos::terrazzo::gateway::client::WriteRequest;
use crate::backend::protos::terrazzo::gateway::client::WriteResponse;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::write::WriteError as WriteErrorImpl;

pub fn write(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: WriteRequest,
) -> impl Future<Output = Result<(), WriteError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(WriteCallback::process(server, client_address, request).await?)
    }
    .instrument(debug_span!("Write"))
}

struct WriteCallback;

impl DistributedCallback for WriteCallback {
    type Request = WriteRequest;
    type Response = ();
    type LocalError = WriteErrorImpl;
    type RemoteError = tonic::Status;

    async fn local(_: &Server, request: WriteRequest) -> Result<(), WriteErrorImpl> {
        let terminal_id = request.terminal.unwrap_or_default().terminal_id.into();
        let span = debug_span!("Write", %terminal_id);
        async {
            debug!("Start");
            defer!(debug!("End"));
            Ok(processes::write::write(&terminal_id, request.data.as_bytes()).await?)
        }
        .instrument(span)
        .await
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        request: WriteRequest,
    ) -> Result<(), tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let request = WriteRequest {
            terminal: Some(TerminalAddress {
                terminal_id: request.terminal.unwrap_or_default().terminal_id,
                via: Some(ClientAddress::of(client_address)),
            }),
            data: request.data,
        };
        let WriteResponse {} = client.write(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{n}] {0}", n = self.name())]
    WriteError(#[from] DistributedCallbackError<WriteErrorImpl, tonic::Status>),
}

impl IsHttpError for WriteError {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        match self {
            Self::WriteError(error) => error.status_code(),
        }
    }
}

impl From<WriteError> for Status {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::WriteError(error) => error.into(),
        }
    }
}

impl From<WriteErrorImpl> for Status {
    fn from(error: WriteErrorImpl) -> Self {
        match error {
            error @ WriteErrorImpl::TerminalNotFound { .. } => Status::not_found(error.to_string()),
            WriteErrorImpl::Write(error) => Status::internal(error.to_string()),
        }
    }
}
