use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;

use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::remotes::list_remotes;
use crate::backend::protos::terrazzo::shared::ListRemotesRequest;
use crate::backend::protos::terrazzo::shared::ListRemotesResponse;
use crate::backend::protos::terrazzo::shared::shared_service_server::SharedService;

#[async_trait]
impl SharedService for ClientServiceImpl {
    async fn list_remotes(
        &self,
        request: Request<ListRemotesRequest>,
    ) -> Result<Response<ListRemotesResponse>, Status> {
        let mut visited = request.into_inner().visited;
        visited.push(self.client_name.to_string());
        let clients = list_remotes(&self.server, visited).await;
        Ok(Response::new(ListRemotesResponse { clients }))
    }
}
