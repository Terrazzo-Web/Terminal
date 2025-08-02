use std::future::ready;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use futures::StreamExt as _;
use futures::stream;
use nameth::NamedEnumValues as _;
use nameth::NamedType as _;
use nameth::nameth;
use pin_project::pin_project;
use prost::bytes::Bytes;
use scopeguard::defer;
use tokio::io::AsyncRead;
use tokio::io::AsyncWriteExt as _;
use tokio::io::ReadBuf;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::oneshot;
use tonic::Status;
use tonic::Streaming;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use super::RequestDataStream;
use super::listeners::EndpointId;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_data_request;
use crate::backend::protos::terrazzo::portforward::port_forward_service_client::PortForwardServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress;

/// Upload data to given endpoint
pub async fn upload(
    server: &Arc<Server>,
    mut download_stream: impl RequestDataStream,
) -> Result<UploadStream, UploadError> {
    let task = async move {
        debug!("Start");
        defer!(debug!("End"));
        let Some(first_message) = download_stream.next().await else {
            return Err(UploadError::EmptyRequest);
        };

        let endpoint = get_endpoint(first_message)?;
        debug!("Uploading data to: {endpoint:?}");

        let remote = endpoint.remote.clone().unwrap_or_default();
        let download_stream =
            UploadCallback::process(server, &remote.via, (endpoint, download_stream)).await?;
        return Ok(download_stream);
    };
    return task.instrument(info_span!("PortForward Upload")).await;
}

fn get_endpoint(
    first_message: Result<PortForwardDataRequest, Status>,
) -> Result<PortForwardEndpoint, UploadError> {
    let PortForwardDataRequest {
        kind: first_message,
    } = first_message.map_err(|status| UploadError::RequestError(Box::new(status)))?;
    match first_message.ok_or(UploadError::MissingEndpoint)? {
        port_forward_data_request::Kind::Endpoint(endpoint) => Ok(endpoint),
        port_forward_data_request::Kind::Data { .. } => Err(UploadError::MissingEndpoint),
    }
}

pub struct UploadCallback<S: RequestDataStream>(PhantomData<S>);

#[pin_project(project = UploadStreamProj)]
pub enum UploadStream {
    Local(#[pin] LocalUploadStream),
    Remote(#[pin] RemoteUploadStream),
}

#[pin_project]
pub struct LocalUploadStream {
    #[pin]
    tcp_stream: OwnedReadHalf,
    buffer: Vec<u8>,
}

#[pin_project]
pub struct RemoteUploadStream(#[pin] Streaming<PortForwardDataResponse>);

impl Stream for UploadStream {
    type Item = Result<PortForwardDataResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            UploadStreamProj::Local(local) => {
                let local = local.project();
                let mut buf = ReadBuf::new(local.buffer);
                let () = std::task::ready!(local.tcp_stream.poll_read(cx, &mut buf))
                    .map_err(|error| Status::aborted(error.to_string()))?;
                let filled = buf.filled();
                if filled.is_empty() {
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Ok(PortForwardDataResponse {
                    data: Bytes::copy_from_slice(filled),
                })))
            }
            UploadStreamProj::Remote(remote) => remote.project().0.poll_next(cx),
        }
    }
}

impl<S: RequestDataStream> DistributedCallback for UploadCallback<S> {
    type Request = (PortForwardEndpoint, S);
    type Response = UploadStream;
    type LocalError = UploadLocalError;
    type RemoteError = UploadRemoteError;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        (endpoint, download_stream): (PortForwardEndpoint, S),
    ) -> Result<UploadStream, UploadRemoteError>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let first_message = PortForwardDataRequest {
                kind: Some(port_forward_data_request::Kind::Endpoint(
                    PortForwardEndpoint {
                        remote: Some(ClientAddress::of(client_address)),
                        ..endpoint
                    },
                )),
            };
            let download_stream = stream::once(ready(first_message))
                .chain(download_stream.filter_map(|next| ready(next.ok())));
            let mut client = PortForwardServiceClient::new(channel);
            let download_stream = client.upload(download_stream).await?;
            Ok(UploadStream::Remote(RemoteUploadStream(
                download_stream.into_inner(),
            )))
        }
        .instrument(info_span!("Remote"))
        .await
    }

    async fn local(
        _server: &Arc<Server>,
        (endpoint, download_stream): (PortForwardEndpoint, S),
    ) -> Result<UploadStream, UploadLocalError> {
        async move {
            debug!("Start");
            defer!(debug!("End"));

            let endpoint_id = EndpointId {
                host: endpoint.host,
                port: endpoint.port,
            };

            let (future_streams, tx) = {
                let mut listeners = super::listeners::listeners();
                let Some(future_streams) = listeners.get_mut(&endpoint_id) else {
                    return Err(UploadLocalError::StreamsNotRegistered(endpoint_id));
                };
                let (tx, rx) = oneshot::channel();
                (std::mem::replace(future_streams, rx), tx)
            };
            let (read_half, write_half) = {
                let streams = future_streams
                    .await
                    .map_err(UploadLocalError::StreamsNotAvailable)?;
                let mut streams = scopeguard::guard(streams, |streams| {
                    let _ = tx.send(streams);
                });
                streams
                    .recv()
                    .await
                    .ok_or(UploadLocalError::NoMoreStreams)?
                    .into_split()
            };

            let requests_task = process_write_half(download_stream, write_half);
            tokio::spawn(requests_task.in_current_span());
            Ok(UploadStream::Local(LocalUploadStream {
                tcp_stream: read_half,
                buffer: vec![0; 8192],
            }))
        }
        .instrument(debug_span!("Local"))
        .await
    }
}

async fn process_write_half(
    mut download_stream: impl RequestDataStream,
    mut write_half: OwnedWriteHalf,
) {
    while let Some(next) = download_stream.next().await {
        match next {
            Ok(PortForwardDataRequest {
                kind: Some(port_forward_data_request::Kind::Endpoint(endpoint)),
            }) => return warn!("Invalid next message is endpoint: {endpoint:?}"),
            Ok(PortForwardDataRequest {
                kind: Some(port_forward_data_request::Kind::Data(bytes)),
            }) => match write_half.write_all(&bytes).await {
                Ok(()) => (),
                Err(error) => return warn!("Failed to write: {error}"),
            },
            Ok(PortForwardDataRequest { kind: None }) => return warn!("Next message is 'None'"),
            Err(error) => return warn!("Failed to get next message: {error}"),
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum UploadLocalError {
    #[error("[{n}] No streams registered under {0:?}", n = self.name())]
    StreamsNotRegistered(EndpointId),

    #[error("[{n}] Failed to get streams: {0}", n = self.name())]
    StreamsNotAvailable(oneshot::error::RecvError),

    #[error("[{n}] No more streams", n = self.name())]
    NoMoreStreams,
}

impl From<UploadLocalError> for Status {
    fn from(error: UploadLocalError) -> Self {
        let code = match error {
            UploadLocalError::StreamsNotRegistered { .. } => tonic::Code::InvalidArgument,
            UploadLocalError::StreamsNotAvailable { .. } => tonic::Code::FailedPrecondition,
            UploadLocalError::NoMoreStreams { .. } => tonic::Code::FailedPrecondition,
        };
        Self::new(code, error.to_string())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct UploadRemoteError(Box<Status>);

impl From<UploadRemoteError> for Status {
    fn from(UploadRemoteError(mut status): UploadRemoteError) -> Self {
        std::mem::replace(status.as_mut(), Status::ok(""))
    }
}

impl From<Status> for UploadRemoteError {
    fn from(status: Status) -> Self {
        Self(Box::new(status))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("[{n}] Empty request", n = Self::type_name())]
    EmptyRequest,

    #[error("[{n}] Failed request: {0}", n = Self::type_name())]
    RequestError(Box<Status>),

    #[error("[{n}] Expected the first message to contain the endpoint", n = Self::type_name())]
    MissingEndpoint,

    #[error("[{n}] {0}", n = Self::type_name())]
    Dispatch(#[from] DistributedCallbackError<UploadLocalError, UploadRemoteError>),
}

impl From<UploadError> for Status {
    fn from(error: UploadError) -> Self {
        let code = match error {
            UploadError::EmptyRequest => tonic::Code::InvalidArgument,
            UploadError::RequestError { .. } => tonic::Code::FailedPrecondition,
            UploadError::MissingEndpoint { .. } => tonic::Code::FailedPrecondition,
            UploadError::Dispatch(error) => return error.into(),
        };
        Self::new(code, error.to_string())
    }
}
