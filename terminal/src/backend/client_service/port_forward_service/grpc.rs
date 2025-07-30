use std::future::ready;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use futures::StreamExt;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tonic::async_trait;
use tracing::debug;

use super::bind::BindCallback;
use super::bind::BindStream;
use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataResponse;
use crate::backend::protos::terrazzo::portforward::port_forward_service_server::PortForwardService;

#[async_trait]
impl PortForwardService for ClientServiceImpl {
    type BindStream = BindStream;

    async fn bind(
        &self,
        requests: Request<Streaming<PortForwardAcceptRequest>>,
    ) -> Result<Response<BindStream>, Status> {
        let mut requests = requests.into_inner();
        let Some(first_request) = requests.next().await else {
            return Err(Status::invalid_argument("Empty request stream"));
        };
        let first_request =
            first_request.map_err(|status| Status::invalid_argument(status.to_string()))?;
        debug!("Port forward request: {first_request:?}");

        let remote = first_request.remote.clone().unwrap_or_default();
        let requests = requests.filter_map(|request| ready(request.ok()));
        let stream =
            BindCallback::process(&self.server, &remote.via, (first_request, requests)).await?;
        return Ok(Response::new(stream));
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
