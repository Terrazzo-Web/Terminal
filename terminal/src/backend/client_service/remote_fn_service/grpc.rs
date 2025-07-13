use tonic::Result;
use tonic::Status;
use tonic::async_trait;

use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::remote_fn_service::dispatch::remote_fn_dispatch;
use crate::backend::protos::terrazzo::remotefn::RemoteFnRequest;
use crate::backend::protos::terrazzo::remotefn::ServerFnResponse;
use crate::backend::protos::terrazzo::remotefn::remote_fn_service_server::RemoteFnService;

#[async_trait]
impl RemoteFnService for ClientServiceImpl {
    async fn call_server_fn(
        &self,
        request: tonic::Request<RemoteFnRequest>,
    ) -> Result<tonic::Response<ServerFnResponse>, Status> {
        let mut request = request.into_inner();
        let address = request.address.get_or_insert_default();
        let address = std::mem::take(&mut address.via);
        let response = remote_fn_dispatch(&self.server, &address, request).await;
        Ok(tonic::Response::new(ServerFnResponse { json: response? }))
    }
}
