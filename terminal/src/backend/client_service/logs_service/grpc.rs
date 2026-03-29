//! Implementation of the Logs service through the gRPC tunnel.

use futures::stream::Empty;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;

use crate::backend::client_service::ClientServiceImpl;
use crate::backend::protos::terrazzo::logs::LogsRequest;
use crate::backend::protos::terrazzo::logs::LogsResponse;
use crate::backend::protos::terrazzo::logs::logs_service_server::LogsService;

#[async_trait]
impl LogsService for ClientServiceImpl {
    type StreamLogsStream = Empty<Result<LogsResponse, Status>>;

    async fn stream_logs(
        &self,
        _request: Request<LogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        Err(Status::unimplemented("LogsService dispatch is not implemented yet"))
    }
}
