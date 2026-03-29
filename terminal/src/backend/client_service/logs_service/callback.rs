use std::sync::Arc;

use futures::StreamExt as _;
use nameth::nameth;
use scopeguard::guard;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::info;
use trz_gateway_server::server::Server;

use super::response::HybridResponseStream;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::protos::terrazzo::logs::LogsRequest;
use crate::backend::protos::terrazzo::logs::logs_service_client::LogsServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;
use crate::logs::event::LogEvent;
use crate::logs::state::LogState;

pub struct LogsCallback;

impl DistributedCallback for LogsCallback {
    type Request = LogsRequest;
    type Response = HybridResponseStream;
    type LocalError = LogsLocalError;
    type RemoteError = Status;

    async fn local(
        _server: Option<&Arc<Server>>,
        _request: LogsRequest,
    ) -> Result<HybridResponseStream, LogsLocalError> {
        local_logs_stream()
            .map(HybridResponseStream::Local)
            .map_err(LogsLocalError)
    }

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        mut request: LogsRequest,
    ) -> Result<HybridResponseStream, Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address = Some(ClientAddressProto::of(client_address));
        let mut client = LogsServiceClient::new(channel);
        let response = client.stream_logs(request).await?.into_inner();
        Ok(HybridResponseStream::Remote(Box::new(response)))
    }
}

fn local_logs_stream() -> Result<BoxedStream<LogEvent, ServerFnError>, ServerFnError> {
    info!("Starting log stream");
    let end = guard((), |_| info!("Ending log stream"));
    let subscription = LogState::get().subscribe();
    let stream = futures::stream::unfold(subscription, |mut subscription| async move {
        let next = if let Some(event) = subscription.backlog.pop_front() {
            Some(event)
        } else {
            subscription.receiver.recv().await
        }?;
        Some(((*next).clone(), subscription))
    });
    let stream = stream.inspect(move |_| {
        let _ = &end;
    });
    Ok(BoxedStream::from(Box::pin(stream.map(Ok))))
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct LogsLocalError(ServerFnError);
