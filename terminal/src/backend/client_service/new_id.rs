use std::convert::Infallible;
use std::future::ready;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::NewIdRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes::next_terminal_id;

pub fn new_id(
    server: &Server,
    client_address: &[impl AsRef<str>],
) -> impl Future<Output = Result<i32, NewIdError>> {
    async {
        info!("Start");
        defer!(info!("Done"));
        Ok(NewIdCallback::process(server, client_address, ()).await?)
    }
    .instrument(info_span!("New ID"))
}

struct NewIdCallback;

impl DistributedCallback for NewIdCallback {
    type Request = ();
    type Response = i32;
    type LocalError = Infallible;
    type RemoteError = tonic::Status;

    fn local(_: &Server, (): ()) -> impl Future<Output = Result<i32, Infallible>> {
        ready(Ok(next_terminal_id()))
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        (): (),
    ) -> Result<i32, tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let t = async move {
            let request = NewIdRequest {
                address: Some(ClientAddress {
                    via: client_address
                        .iter()
                        .map(|x| x.as_ref().to_owned())
                        .collect(),
                }),
            };
            let response = client.new_id(request).await;
            Ok(response?.get_ref().next)
        };
        t.await
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
    #[error("[{n}] {0}", n = self.name())]
    NewIdError(#[from] DistributedCallbackError<Infallible, tonic::Status>),
}

impl IsHttpError for NewIdError {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        match self {
            Self::NewIdError(error) => error.status_code(),
        }
    }
}
