use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tonic::async_trait;

use super::bind::BindStream;
use crate::backend::client_service::ClientServiceImpl;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_service_server::PortForwardService;

#[async_trait]
impl PortForwardService for ClientServiceImpl {
    type BindStream = BindStream;

    async fn bind(
        &self,
        requests: Request<Streaming<PortForwardEndpoint>>,
    ) -> Result<Response<BindStream>, Status> {
        let stream = super::bind::dispatch(&self.server, requests.into_inner()).await?;
        Ok(Response::new(stream))
    }

    type DownloadStream = DownloadStream;

    async fn download(
        &self,
        _request: Request<tonic::Streaming<PortForwardDataRequest>>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        todo!()
    }

    type UploadStream = UploadStream;

    async fn upload(
        &self,
        _request: Request<tonic::Streaming<PortForwardDataRequest>>,
    ) -> Result<Response<Self::UploadStream>, Status> {
        todo!()
    }
}

pub struct DownloadStream;
pub struct UploadStream;

impl Stream for DownloadStream {
    type Item = Result<PortForwardDataResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

impl Stream for UploadStream {
    type Item = Result<PortForwardDataResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}
