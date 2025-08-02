use std::future::ready;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::Sink;
use futures::Stream;
use futures::StreamExt as _;
use futures::stream;
use nameth::NamedEnumValues as _;
use nameth::NamedType as _;
use nameth::nameth;
use pin_project::pin_project;
use prost::bytes::Bytes;
use scopeguard::defer;
use tokio::io::AsyncRead as _;
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

pub async fn download(
    server: &Arc<Server>,
    mut upload_stream: impl RequestDataStream,
) -> Result<DownloadStream, DownloadError> {
    let task = async move {
        debug!("Start");
        defer!(debug!("End"));
        let Some(first_message) = upload_stream.next().await else {
            return Err(DownloadError::EmptyRequest);
        };

        let endpoint = get_endpoint(first_message)?;
        debug!("Downloading data from: {endpoint:?}");

        let remote = endpoint.remote.clone().unwrap_or_default();
        let download_stream =
            DownloadCallback::process(server, &remote.via, (endpoint, upload_stream)).await?;
        return Ok(download_stream);
    };
    return task.instrument(info_span!("PortForward Download")).await;
}

fn get_endpoint(
    first_message: Result<PortForwardDataRequest, Status>,
) -> Result<PortForwardEndpoint, DownloadError> {
    let PortForwardDataRequest {
        kind: first_message,
    } = first_message.map_err(|status| DownloadError::RequestError(Box::new(status)))?;
    match first_message.ok_or(DownloadError::MissingEndpoint)? {
        port_forward_data_request::Kind::Endpoint(endpoint) => Ok(endpoint),
        port_forward_data_request::Kind::Data { .. } => Err(DownloadError::MissingEndpoint),
    }
}

pub struct DownloadCallback<S: RequestDataStream>(PhantomData<S>);

#[pin_project(project = DownloadStreamProj)]
pub enum DownloadStream {
    Local(#[pin] LocalDownloadStream),
    Remote(#[pin] RemoteDownloadStream),
}

#[pin_project]
pub struct LocalDownloadStream {
    #[pin]
    tcp_stream: OwnedReadHalf,
    buffer: Vec<u8>,
}

#[pin_project]
pub struct RemoteDownloadStream(#[pin] Streaming<PortForwardDataResponse>);

impl Stream for DownloadStream {
    type Item = Result<PortForwardDataResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            DownloadStreamProj::Local(local) => {
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
            DownloadStreamProj::Remote(remote) => remote.project().0.poll_next(cx),
        }
    }
}

impl<S: RequestDataStream> DistributedCallback for DownloadCallback<S> {
    type Request = (PortForwardEndpoint, S);
    type Response = DownloadStream;
    type LocalError = DownloadLocalError;
    type RemoteError = DownloadRemoteError;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        (endpoint, upload_stream): (PortForwardEndpoint, S),
    ) -> Result<DownloadStream, DownloadRemoteError>
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
            let upload_stream = stream::once(ready(first_message))
                .chain(upload_stream.filter_map(|next| ready(next.ok())));
            let mut client = PortForwardServiceClient::new(channel);
            let download_stream = client.download(upload_stream).await?;
            Ok(DownloadStream::Remote(RemoteDownloadStream(
                download_stream.into_inner(),
            )))
        }
        .instrument(info_span!("Remote"))
        .await
    }

    async fn local(
        _server: &Arc<Server>,
        (endpoint, upload_stream): (PortForwardEndpoint, S),
    ) -> Result<DownloadStream, DownloadLocalError> {
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
                    return Err(DownloadLocalError::StreamsNotRegistered(endpoint_id));
                };
                let (tx, rx) = oneshot::channel();
                (std::mem::replace(future_streams, rx), tx)
            };
            let (read_half, write_half) = {
                let streams = future_streams
                    .await
                    .map_err(DownloadLocalError::StreamsNotAvailable)?;
                let mut streams = scopeguard::guard(streams, |streams| {
                    let _ = tx.send(streams);
                });
                streams
                    .recv()
                    .await
                    .ok_or(DownloadLocalError::NoMoreStreams)?
                    .into_split()
            };

            let requests_task = process_write_half(upload_stream, write_half);
            tokio::spawn(requests_task.in_current_span());
            Ok(DownloadStream::Local(LocalDownloadStream {
                tcp_stream: read_half,
                buffer: vec![0; 8192],
            }))
        }
        .instrument(debug_span!("Local"))
        .await
    }
}

async fn process_write_half(
    mut upload_stream: impl RequestDataStream,
    mut write_half: OwnedWriteHalf,
) {
    while let Some(next) = upload_stream.next().await {
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

#[pin_project]
struct WriteSink<T, WO, W> {
    #[pin]
    writer: T,
    operation: Option<WO>,
    write_all: W,
}

impl<T, WO, W> Sink<Bytes> for WriteSink<T, WO, W>
where
    T: tokio::io::AsyncWrite,
    W: Fn(Pin<&mut T>, &[u8]) -> WO,
    WO: Future<Output = std::io::Result<()>>,
{
    type Error = std::io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Always ready to accept new data
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        let this = self.project();
        let write_all = this.write_all;
        *this.operation = Some(write_all(this.writer, &item));
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadLocalError {
    #[error("[{n}] No streams registered under {0:?}", n = self.name())]
    StreamsNotRegistered(EndpointId),

    #[error("[{n}] Failed to get streams: {0}", n = self.name())]
    StreamsNotAvailable(oneshot::error::RecvError),

    #[error("[{n}] No more streams", n = self.name())]
    NoMoreStreams,
}

impl From<DownloadLocalError> for Status {
    fn from(error: DownloadLocalError) -> Self {
        let code = match error {
            DownloadLocalError::StreamsNotRegistered { .. } => tonic::Code::InvalidArgument,
            DownloadLocalError::StreamsNotAvailable { .. } => tonic::Code::FailedPrecondition,
            DownloadLocalError::NoMoreStreams { .. } => tonic::Code::FailedPrecondition,
        };
        Self::new(code, error.to_string())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct DownloadRemoteError(Box<Status>);

impl From<DownloadRemoteError> for Status {
    fn from(DownloadRemoteError(mut status): DownloadRemoteError) -> Self {
        std::mem::replace(status.as_mut(), Status::ok(""))
    }
}

impl From<Status> for DownloadRemoteError {
    fn from(status: Status) -> Self {
        Self(Box::new(status))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("[{n}] Empty request", n = Self::type_name())]
    EmptyRequest,

    #[error("[{n}] Failed request: {0}", n = Self::type_name())]
    RequestError(Box<Status>),

    #[error("[{n}] Expected the first message to contain the endpoint", n = Self::type_name())]
    MissingEndpoint,

    #[error("[{n}] {0}", n = Self::type_name())]
    Dispatch(#[from] DistributedCallbackError<DownloadLocalError, DownloadRemoteError>),
}

impl From<DownloadError> for Status {
    fn from(error: DownloadError) -> Self {
        let code = match error {
            DownloadError::EmptyRequest => tonic::Code::InvalidArgument,
            DownloadError::RequestError { .. } => tonic::Code::FailedPrecondition,
            DownloadError::MissingEndpoint { .. } => tonic::Code::FailedPrecondition,
            DownloadError::Dispatch(error) => return error.into(),
        };
        Self::new(code, error.to_string())
    }
}
