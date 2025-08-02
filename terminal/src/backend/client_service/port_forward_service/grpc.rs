use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tonic::async_trait;

use super::bind::BindStream;
use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::port_forward_service::download::DownloadStream;
use crate::backend::client_service::port_forward_service::upload::UploadStream;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_service_server::PortForwardService;

#[async_trait]
impl PortForwardService for ClientServiceImpl {
    type BindStream = BindStream;

    async fn bind(
        &self,
        requests: Request<Streaming<PortForwardEndpoint>>,
    ) -> Result<Response<BindStream>, Status> {
        let upload_stream = super::bind::dispatch(&self.server, requests.into_inner()).await?;
        Ok(Response::new(upload_stream))
    }

    type DownloadStream = DownloadStream;

    async fn download(
        &self,
        requests: Request<tonic::Streaming<PortForwardDataRequest>>,
    ) -> Result<Response<DownloadStream>, Status> {
        let download_stream =
            super::download::download(&self.server, requests.into_inner()).await?;
        Ok(Response::new(download_stream))
    }

    type UploadStream = UploadStream;

    async fn upload(
        &self,
        requests: Request<tonic::Streaming<PortForwardDataRequest>>,
    ) -> Result<Response<UploadStream>, Status> {
        let upload_stream = super::upload::upload(&self.server, requests.into_inner()).await?;
        Ok(Response::new(upload_stream))
    }
}
