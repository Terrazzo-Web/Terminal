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
use tracing::info_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use super::convert::Impossible;
use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::AckRequest;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::Empty;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::backend::throttling_stream;

pub fn ack(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: AckRequest,
) -> impl Future<Output = Result<(), AckError>> {
    let terminal_id = request
        .terminal
        .as_ref()
        .map(|t| t.terminal_id.as_str())
        .unwrap_or_default();
    let span = info_span!("Ack", %terminal_id);
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(AckCallback::process(server, client_address, request).await?)
    }
    .instrument(span)
}

struct AckCallback;

impl DistributedCallback for AckCallback {
    type Request = AckRequest;
    type Response = ();
    type LocalError = Impossible;
    type RemoteError = Status;

    async fn local(_: &Server, request: AckRequest) -> Result<(), Impossible> {
        let terminal_id = request.terminal.unwrap_or_default().terminal_id.into();
        throttling_stream::ack(&terminal_id, request.ack as usize);
        Ok(())
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: AckRequest,
    ) -> Result<(), Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.terminal.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let Empty {} = client.ack(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum AckError {
    #[error("[{n}] {0}", n = self.name())]
    AckError(#[from] DistributedCallbackError<Impossible, Status>),
}

impl IsHttpError for AckError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AckError(error) => error.status_code(),
        }
    }
}

impl From<AckError> for Status {
    fn from(error: AckError) -> Self {
        match error {
            AckError::AckError(error) => error.into(),
        }
    }
}
