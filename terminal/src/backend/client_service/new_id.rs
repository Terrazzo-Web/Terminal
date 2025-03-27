use std::convert::Infallible;
use std::future::ready;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tonic::transport::Channel;
use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use trz_gateway_server::connection::pending_requests::PendingRequests;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::NewIdRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes::next_terminal_id;

pub fn new_id(
    server: &Server,
    client_address: &[impl AsRef<str> + Send + Sync],
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

    async fn remote(
        mut client: ClientServiceClient<PendingRequests<Channel>>,
        client_address: &[impl AsRef<str> + Send + Sync],
        (): (),
    ) -> Result<i32, tonic::Status> {
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
        check_send(&t);
        t.await
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
    #[error("[{n}] {0}", n = self.name())]
    NewIdError(#[from] DistributedCallbackError<Infallible, tonic::Status>),
}

fn check_send<T: Send>(_: &T) {}
fn check_sync<T: Sync>(_: &T) {}
