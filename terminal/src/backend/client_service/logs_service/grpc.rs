//! Implementation of the Logs service through the gRPC tunnel.

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;

use super::dispatch::logs_dispatch;
use super::response::remote::RemoteResponseStream;
use crate::backend::client_service::ClientServiceImpl;
use crate::backend::protos::terrazzo::logs::LogsRequest;
use crate::backend::protos::terrazzo::logs::logs_service_server::LogsService;

#[async_trait]
impl LogsService for ClientServiceImpl {
    type StreamLogsStream = RemoteResponseStream;

    async fn stream_logs(
        &self,
        request: Request<LogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let response = logs_dispatch(request.into_inner())
            .await
            .map_err(Status::from)?;
        Ok(Response::new(RemoteResponseStream(response)))
    }
}
