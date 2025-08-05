use std::future::ready;
use std::sync::Arc;

use futures::StreamExt as _;
use nameth::nameth;
use server_fn::ServerFnError;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use trz_gateway_server::server::Server;

use crate::backend::client_service::notify_service::request::HybridRequestStream;
use crate::backend::client_service::notify_service::request::remote::RemoteRequestStream;
use crate::backend::client_service::notify_service::response::HybridResponseStream;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::protos::terrazzo::notify::NotifyRequest as NotifyRequestProto;
use crate::backend::protos::terrazzo::notify::notify_request::RequestType as RequestTypeProto;
use crate::backend::protos::terrazzo::notify::notify_service_client::NotifyServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;
use crate::text_editor::notify::service::notify as notify_local;

pub struct NotifyCallback;

impl DistributedCallback for NotifyCallback {
    type Request = HybridRequestStream;
    type Response = HybridResponseStream;
    type LocalError = NotifyLocalError;
    type RemoteError = Status;

    async fn local(
        _server: &Arc<Server>,
        request: HybridRequestStream,
    ) -> Result<HybridResponseStream, NotifyLocalError> {
        notify_local(request.into())
            .map_err(NotifyLocalError)
            .map(HybridResponseStream::Local)
    }

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        request: HybridRequestStream,
    ) -> Result<HybridResponseStream, Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        let client_address = ClientAddressProto::of(client_address);
        let request = RemoteRequestStream(request).filter_map(|request| ready(request.ok()));
        let request = futures::stream::once(ready(NotifyRequestProto {
            request_type: Some(RequestTypeProto::Address(client_address)),
        }))
        .chain(request);
        let mut client = NotifyServiceClient::new(channel);
        let response = client.notify(request).await?.into_inner();
        Ok(HybridResponseStream::Remote(Box::new(response)))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct NotifyLocalError(ServerFnError);
