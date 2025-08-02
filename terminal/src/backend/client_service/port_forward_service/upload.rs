use std::future::ready;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::AsyncWriteExt as _;
use futures::SinkExt as _;
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
use tokio::io::AsyncWrite as _;
use tokio::io::ReadBuf;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
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
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_data_request;
use crate::backend::protos::terrazzo::portforward::port_forward_service_client::PortForwardServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress;

/// Upload data from listener
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
        let upload_stream =
            UploadCallback::process(server, &remote.via, (endpoint, download_stream)).await?;
        return Ok(upload_stream);
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
            let upload_stream = client.upload(download_stream).await?;
            Ok(UploadStream::Remote(RemoteUploadStream(
                upload_stream.into_inner(),
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

            let PortForwardEndpoint { host, port, .. } = endpoint;
            let (read_half, write_half) = TcpStream::connect(format!("{host}:{port}"))
                .await
                .map_err(UploadLocalError::Connect)?
                .into_split();

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
    write_half: OwnedWriteHalf,
) {
    let mut sink = WriteHalf(write_half).into_sink::<Bytes>().buffer(8192);
    while let Some(next) = download_stream.next().await {
        match next {
            Ok(PortForwardDataRequest {
                kind: Some(port_forward_data_request::Kind::Endpoint(endpoint)),
            }) => {
                warn!("Invalid next message is endpoint: {endpoint:?}");
                break;
            }
            Ok(PortForwardDataRequest {
                kind: Some(port_forward_data_request::Kind::Data(bytes)),
            }) => match sink.feed(bytes).await {
                Ok(()) => {}
                Err(error) => {
                    warn!("Failed to write: {error}");
                    return;
                }
            },
            Ok(PortForwardDataRequest { kind: None }) => {
                warn!("Next message is 'None'");
                break;
            }
            Err(error) => {
                warn!("Failed to get next message: {error}");
                break;
            }
        }
    }
    match sink.flush().await {
        Ok(()) => {}
        Err(error) => return warn!("Failed to flush: {error}"),
    }
}

#[pin_project]
struct WriteHalf(#[pin] OwnedWriteHalf);

impl futures::AsyncWrite for WriteHalf {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().0.poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        self.project().0.poll_write_vectored(cx, bufs)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum UploadLocalError {
    #[error("[{n}] Failed to connect: {0}", n = self.name())]
    Connect(std::io::Error),
}

impl From<UploadLocalError> for Status {
    fn from(error: UploadLocalError) -> Self {
        let code = match error {
            UploadLocalError::Connect { .. } => tonic::Code::Aborted,
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
