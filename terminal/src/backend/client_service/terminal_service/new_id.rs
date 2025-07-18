use std::future::ready;

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

use crate::backend::client_service::convert::Impossible;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::shared::ClientAddress;
use crate::backend::protos::terrazzo::terminal::NewIdRequest;
use crate::backend::protos::terrazzo::terminal::terminal_service_client::TerminalServiceClient;
use crate::processes::next_terminal_id;

pub fn new_id(
    server: &Server,
    client_address: &[impl AsRef<str>],
) -> impl Future<Output = Result<i32, NewIdError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(NewIdCallback::process(server, client_address, ()).await?)
    }
    .instrument(debug_span!("New ID"))
}

struct NewIdCallback;

impl DistributedCallback for NewIdCallback {
    type Request = ();
    type Response = i32;
    type LocalError = Impossible;
    type RemoteError = Status;

    fn local(_: &Server, (): ()) -> impl Future<Output = Result<i32, Impossible>> {
        ready(Ok(next_terminal_id()))
    }

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        (): (),
    ) -> Result<i32, Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let request = NewIdRequest {
            address: Some(ClientAddress::of(client_address)),
        };
        let mut client = TerminalServiceClient::new(channel);
        let response = client.new_id(request).await;
        let id = response?.get_ref().next;
        debug!(id, "Allocated ID");
        Ok(id)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
    #[error("[{n}] {0}", n = self.name())]
    NewIdError(#[from] DistributedCallbackError<Impossible, Status>),
}

impl IsHttpError for NewIdError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NewIdError(error) => error.status_code(),
        }
    }
}

impl From<NewIdError> for Status {
    fn from(error: NewIdError) -> Self {
        match error {
            NewIdError::NewIdError(error) => error.into(),
        }
    }
}
