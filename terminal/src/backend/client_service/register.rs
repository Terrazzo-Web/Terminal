use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
use tonic::Status;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::api::RegisterTerminalMode;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::io::HybridReader;

pub async fn register(
    server: &Server,
    mut request: RegisterTerminalRequest,
) -> Result<HybridReader, Status> {
    let terminal_def = request.def.get_or_insert_default();
    let client_address = terminal_def.client_address().to_vec();
    async move {
        info!("Start");
        defer!(info!("Done"));
        let stream = RegisterCallback::process(server, &client_address, request).await?;
        Ok(stream)
    }
    .instrument(info_span!("Register"))
    .await
}

struct RegisterCallback;

impl DistributedCallback for RegisterCallback {
    type Request = RegisterTerminalRequest;
    type Response = HybridReader;
    type LocalError = Status;
    type RemoteError = Status;

    async fn local(
        server: &Server,
        request: RegisterTerminalRequest,
    ) -> Result<HybridReader, Status> {
        let mode = request.mode().try_into()?;
        let def = request.def.ok_or_else(|| Status::invalid_argument("def"))?;
        let stream = processes::stream::open_stream(server, def.into(), |_| async {
            match mode {
                RegisterTerminalMode::Create => ProcessIO::open().await,
                RegisterTerminalMode::Reopen => Err(OpenProcessError::NotFound),
            }
        })
        .await;
        let stream = stream.map_err(|error| Status::internal(error.to_string()))?;
        return Ok(HybridReader::Local(stream));
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: RegisterTerminalRequest,
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
        Ok(HybridReader::Remote(stream))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RegisterStreamError {
    #[error("[{n}] {0}", n = self.name())]
    RegisterStreamError(#[from] DistributedCallbackError<Status, Status>),
}
