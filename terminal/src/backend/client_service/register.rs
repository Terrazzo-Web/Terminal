use bytes::Bytes;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
use tonic::Status;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::Body;
use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use super::routing::StdError;
use crate::api::RegisterTerminalMode;
use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::MaybeString;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalMode as RegisterTerminalModeProto;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::io::HybridReader;

pub async fn register(
    server: &Server,
    request: RegisterTerminalRequest,
) -> Result<HybridReader, Status> {
    let client_address = request
        .def
        .as_ref()
        .and_then(|def| def.via.as_ref())
        .map(|client_address| client_address.via.as_slice())
        .unwrap_or_default()
        .to_vec();
    async move {
        info!("Start");
        defer!(info!("Done"));
        let stream = RegisterCallback::process(server, &client_address, request).await;
        Ok(stream.unwrap())
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

    fn local(
        server: &Server,
        request: RegisterTerminalRequest,
    ) -> impl Future<Output = Result<HybridReader, Status>> {
        async {
            let mode = match request.mode() {
                RegisterTerminalModeProto::Unspecified => {
                    return Err(Status::invalid_argument("mode"));
                }
                RegisterTerminalModeProto::Create => RegisterTerminalMode::Create,
                RegisterTerminalModeProto::Reopen => RegisterTerminalMode::Reopen,
            };
            let def = request.def.ok_or_else(|| Status::invalid_argument("def"))?;
            let stream = processes::stream::open_stream(
                server,
                TerminalDef {
                    id: def.id.into(),
                    title: TabTitle {
                        shell_title: def.shell_title,
                        override_title: def.override_title.map(|s: MaybeString| s.s),
                    },
                    order: def.order,
                    via: def
                        .via
                        .ok_or_else(|| Status::invalid_argument("via"))?
                        .into(),
                },
                |_| async {
                    match mode {
                        RegisterTerminalMode::Create => ProcessIO::open().await,
                        RegisterTerminalMode::Reopen => Err(OpenProcessError::NotFound),
                    }
                },
            )
            .await;
            let stream = stream.map_err(|error| Status::internal(error.to_string()))?;
            return Ok(HybridReader::Local(stream));
        }
    }

    fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: RegisterTerminalRequest,
    ) -> impl Future<Output = Result<HybridReader, Status>>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        async move {
            let def = request
                .def
                .as_mut()
                .ok_or_else(|| Status::invalid_argument("def"))?;
            def.via = Some(ClientAddress {
                via: client_address
                    .iter()
                    .map(|x| x.as_ref().to_owned())
                    .collect(),
            });
            let stream = client.register(request).await?.into_inner();
            Ok(HybridReader::Remote(stream))
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RegisterStreamError {
    #[error("[{n}] {0}", n = self.name())]
    RegisterStreamError(#[from] DistributedCallbackError<Status, Status>),
}
