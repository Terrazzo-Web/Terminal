//#![allow(unused)]

use std::future::ready;

use futures::StreamExt as _;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use notify::RecommendedWatcher;
use server_fn::ServerFnError;
use tokio::sync::mpsc;
use tonic::Status;
use tonic::codegen::StdError;
use trz_gateway_server::server::Server;

use self::response::HybridResponseStream;
use super::routing::DistributedCallback;
use crate::backend::protos::terrazzo::gateway::client::NotifyRequest as NotifyRequestProto;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

mod request;
mod response;

pub enum HybridWatcher {
    Local(RecommendedWatcher),
    Remote(mpsc::Sender<NotifyRequestProto>),
}

struct NotifyCallback;

impl DistributedCallback for NotifyCallback {
    type Request = NotifyRequestProto;
    type Response = HybridResponseStream;
    type LocalError = LocalNotifyError;
    type RemoteError = Status;

    async fn local(
        server: &Server,
        request: NotifyRequestProto,
    ) -> Result<HybridResponseStream, LocalNotifyError> {
        crate::text_editor::notify::service::notify(request)
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        request: NotifyRequestProto,
    ) -> Result<HybridResponseStream, Status>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: tonic::transport::Body<Data = server_fn::Bytes> + Send + 'static,
        <T::ResponseBody as tonic::transport::Body>::Error: Into<StdError> + Send,
    {
        let request = request.filter_map(|item| ready(item.ok()));
        let remote_stream = client.notify(request).await?;
        let remote_stream = remote_stream.into_inner();
        Ok(HybridResponseStream::Remote { remote_stream })
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum LocalNotifyError {
    #[error("[{n}] {0}", n = self.name())]
    LocalNotifyError(ServerFnError),
}
