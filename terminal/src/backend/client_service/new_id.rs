use std::convert::Infallible;
use std::future::ready;

use bytes::Bytes;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
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
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::NewIdRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes::next_terminal_id;

pub async fn new_id(
    server: &Server,
    client_address: &[impl AsRef<str>],
) -> Result<i32, NewIdError> {
    async {
        info!("Start");
        defer!(info!("Done"));
        Ok(NewIdCallback::process(server, client_address, ()).await?)
    }
    .instrument(info_span!("New ID"))
    .await
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

    fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        (): (),
    ) -> impl Future<Output = Result<i32, tonic::Status>>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let request = NewIdRequest {
            address: Some(ClientAddress {
                via: client_address
                    .iter()
                    .map(|x| x.as_ref().to_owned())
                    .collect(),
            }),
        };
        async move { Ok(client.new_id(request).await?.get_ref().next) }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
    #[error("[{n}] {0}", n = self.name())]
    NewIdError(#[from] DistributedCallbackError<Infallible, tonic::Status>),
}
