use tonic::async_trait;
use tracing::info;

use super::protos::terrazzo::gateway::client::NewIdRequest;
use super::protos::terrazzo::gateway::client::NewIdResponse;
use super::protos::terrazzo::gateway::client::client_service_server::ClientService;
use crate::processes::next_terminal_id;

pub struct ClientServiceImpl;

#[async_trait]
impl ClientService for ClientServiceImpl {
    async fn new_id(
        &self,
        _: tonic::Request<NewIdRequest>,
    ) -> std::result::Result<tonic::Response<NewIdResponse>, tonic::Status> {
        let next = next_terminal_id();
        info!("Allocate new ID: {next}");
        Ok(tonic::Response::new(NewIdResponse { next }))
    }
}
