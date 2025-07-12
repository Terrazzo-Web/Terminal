//! Implementation of the Notify service through the gRPC tunnel.

use tonic::Request;
use tonic::Response;
use tonic::Result;
use tonic::Status;
use tonic::Streaming;
use tonic::async_trait;

use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::notify_service::dispatch::notify_dispatch;
use crate::backend::client_service::notify_service::response::remote::RemoteResponseStream;
use crate::backend::protos::terrazzo::notify::NotifyRequest;
use crate::backend::protos::terrazzo::notify::notify_service_server::NotifyService;

#[async_trait]
impl NotifyService for ClientServiceImpl {
    type NotifyStream = RemoteResponseStream;

    async fn notify(
        &self,
        request: Request<Streaming<NotifyRequest>>,
    ) -> Result<Response<Self::NotifyStream>, Status> {
        notify_dispatch(request.into_inner().into())
            .map(|response| RemoteResponseStream(response).into())
            .map_err(Status::from)
    }
}
