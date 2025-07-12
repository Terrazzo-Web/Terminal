use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::http::StatusCode;
use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
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
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::api::shared::terminal_schema::RegisterTerminalMode;
use crate::api::shared::terminal_schema::STREAMING_WINDOW_SIZE;
use crate::api::shared::terminal_schema::TerminalDef;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::backend::throttling_stream::ThrottleProcessOutput;
use crate::processes;
use crate::processes::io::HybridReader;

pub async fn register(
    my_client_name: Option<ClientName>,
    server: &Server,
    mut request: RegisterTerminalRequest,
) -> Result<HybridReader, RegisterStreamError> {
    let terminal_def = request.def.get_or_insert_default();
    let client_address = terminal_def.client_address().to_vec();
    async move {
        info!("Start");
        defer!(info!("Done"));
        let stream =
            RegisterCallback::process(server, &client_address, (my_client_name, request)).await;
        let stream = stream.map_err(|error| error.map_local(Box::new))?;
        Ok(stream)
    }
    .instrument(info_span!("Register"))
    .await
}

struct RegisterCallback;

impl DistributedCallback for RegisterCallback {
    type Request = (Option<ClientName>, RegisterTerminalRequest);
    type Response = HybridReader;
    type LocalError = RegisterStreamError;
    type RemoteError = Status;

    async fn local(
        server: &Server,
        (my_client_name, request): (Option<ClientName>, RegisterTerminalRequest),
    ) -> Result<HybridReader, RegisterStreamError> {
        let mode = request.mode().try_into()?;
        let def = request.def.ok_or_else(|| Status::invalid_argument("def"))?;
        let def = TerminalDef::from(def);
        let terminal_id = def.address.id.clone();
        let stream = processes::stream::open_stream(
            server,
            def,
            mode == RegisterTerminalMode::Create,
            |_| async move {
                match mode {
                    RegisterTerminalMode::Create => {
                        ProcessIO::open(
                            my_client_name.map(|s| s.to_string()),
                            STREAMING_WINDOW_SIZE,
                        )
                        .await
                    }
                    RegisterTerminalMode::Reopen => Err(OpenProcessError::NotFound),
                }
            },
        )
        .await;
        let stream = stream.map_err(|error| Status::internal(error.to_string()))?;
        let stream = ThrottleProcessOutput::new(terminal_id, stream);
        return Ok(HybridReader::Local(stream));
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        (_, mut request): (Option<ClientName>, RegisterTerminalRequest),
    ) -> Result<HybridReader, Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let def = request.def.as_mut();
        let def = def.ok_or_else(|| Status::invalid_argument("def"))?;
        let address = def.address.get_or_insert_default();
        address.via = Some(ClientAddress::of(client_address));
        let stream = client.register(request).await?.into_inner();
        Ok(HybridReader::Remote(Box::new(stream)))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RegisterStreamError {
    #[error("[{n}] {0}", n = self.name())]
    RegisterStreamError(#[from] DistributedCallbackError<Box<RegisterStreamError>, Status>),

    #[error("[{n}] {0}", n = self.name())]
    Grpc(#[from] Status),
}

impl IsHttpError for RegisterStreamError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::RegisterStreamError(error) => error.status_code(),
            Self::Grpc(error) => error.status_code(),
        }
    }
}

impl From<RegisterStreamError> for Status {
    fn from(error: RegisterStreamError) -> Self {
        match error {
            RegisterStreamError::RegisterStreamError(error) => error.into(),
            RegisterStreamError::Grpc(status) => status,
        }
    }
}

impl From<Box<RegisterStreamError>> for Status {
    fn from(error: Box<RegisterStreamError>) -> Self {
        Status::from(*error)
    }
}
