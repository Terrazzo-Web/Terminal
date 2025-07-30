use std::collections::hash_map;
use std::future::ready;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::net::ToSocketAddrs as _;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use futures::StreamExt;
use nameth::NamedEnumValues as _;
use nameth::NamedType;
use nameth::nameth;
use pin_project::pin_project;
use scopeguard::defer;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tonic::Status;
use tonic::Streaming;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use tracing::warn;
use trz_gateway_common::handle::ServerHandle;
use trz_gateway_server::server::Server;

use crate::backend::client_service::port_forward_service::listeners::EndpointId;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptResponse;
use crate::backend::protos::terrazzo::portforward::port_forward_service_client::PortForwardServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress;

pub struct BindCallback<S: Stream<Item = PortForwardAcceptRequest>>(PhantomData<S>);

#[pin_project(project = BindStreamProj)]
pub enum BindStream {
    Local(#[pin] LocalBindStream),
    Remote(#[pin] RemoteBindStream),
}

#[pin_project]
pub struct LocalBindStream(#[pin] mpsc::Receiver<PortForwardAcceptResponse>);

#[pin_project]
pub struct RemoteBindStream(#[pin] Streaming<PortForwardAcceptResponse>);

impl Stream for BindStream {
    type Item = Result<PortForwardAcceptResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            BindStreamProj::Local(local) => {
                match std::task::ready!(local.project().0.poll_recv(cx)) {
                    Some(response) => Some(Ok(response)),
                    None => None,
                }
                .into()
            }
            BindStreamProj::Remote(remote) => remote.project().0.poll_next(cx),
        }
    }
}

impl<S: Stream<Item = PortForwardAcceptRequest> + Send + Unpin + 'static> DistributedCallback
    for BindCallback<S>
{
    type Request = (PortForwardAcceptRequest, S);
    type Response = BindStream;
    type LocalError = BindLocalError;
    type RemoteError = BindRemoteError;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        (first_request, requests): (PortForwardAcceptRequest, S),
    ) -> Result<BindStream, BindRemoteError>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let first_request = PortForwardAcceptRequest {
                remote: Some(ClientAddress::of(client_address)),
                ..first_request
            };
            let requests = futures::stream::once(ready(first_request)).chain(requests);
            let mut client = PortForwardServiceClient::new(channel);
            let response = client.bind(requests).await?;
            Ok(BindStream::Remote(RemoteBindStream(response.into_inner())))
        }
        .instrument(debug_span!("PortForward remote"))
        .await
    }

    async fn local(
        _server: &Server,
        (first_request, requests): (PortForwardAcceptRequest, S),
    ) -> Result<BindStream, BindLocalError> {
        let mut requests = futures::stream::once(ready(first_request)).chain(requests);
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let mut next = requests.next().await;
            let (notify_tx, notify_rx) = mpsc::channel(3);
            while let Some(request) = next.take() {
                let span = debug_span!("Request", host = request.host, port = request.port);
                next = async {
                    debug!("Port forward request: {request:?}");
                    defer!(debug!("Port forward shutdown"));
                    let shutdown = process_request(&notify_tx, request).await?;
                    let next = requests.next().await;
                    let () = shutdown.await;
                    return Ok(next);
                }
                .instrument(span)
                .await?;
            }
            Ok(BindStream::Local(LocalBindStream(notify_rx)))
        }
        .instrument(debug_span!("PortForward"))
        .await
    }
}

async fn process_request(
    notify_tx: &mpsc::Sender<PortForwardAcceptResponse>,
    request: PortForwardAcceptRequest,
) -> Result<impl Future<Output = ()>, BindLocalError> {
    let endpoint_id = EndpointId {
        host: request.host,
        port: request.port,
    };
    let addresses = format!("{}:{}", endpoint_id.host, endpoint_id.port)
        .to_socket_addrs()
        .map_err(BindLocalError::Hostname)?;

    let mut handles = vec![];
    let (streams_tx, streams_rx) = mpsc::channel(3);
    match super::listeners::listeners().entry(endpoint_id.clone()) {
        hash_map::Entry::Occupied(_occupied) => {
            return Err(BindLocalError::EndpointInUse(endpoint_id));
        }
        hash_map::Entry::Vacant(entry) => {
            entry.insert(streams_rx);
        }
    }
    for address in addresses {
        let (shutdown, terminated, handle) = ServerHandle::new(format!("port forward"));
        handles.push(handle);
        process_socket_address(
            address,
            notify_tx.clone(),
            streams_tx.clone(),
            shutdown,
            terminated,
        )
        .await?;
    }

    let shutdown = async move {
        for handle in handles {
            let () = handle
                .stop("PortForward request shutdown")
                .await
                .unwrap_or_else(|error| warn!("{error}"));
        }
    };

    Ok(shutdown)
}

async fn process_socket_address(
    address: SocketAddr,
    notify: mpsc::Sender<PortForwardAcceptResponse>,
    streams: mpsc::Sender<TcpStream>,
    shutdown: impl Future<Output = ()> + Send + 'static,
    terminated: oneshot::Sender<()>,
) -> Result<(), BindLocalError> {
    let listener = TcpListener::bind(address)
        .await
        .map_err(BindLocalError::Bind)?;
    let task = async move {
        debug!("Start");
        defer!(debug!("End"));
        let listener_task = process_listener(listener, notify, streams);
        match futures::future::select(Box::pin(listener_task), Box::pin(shutdown)).await {
            futures::future::Either::Left((Ok(()), _)) => {
                warn!("The listener task stopped, but it's an infinite loop")
            }
            futures::future::Either::Left((Err(error), _)) => {
                warn!("The listener task failed: {error}")
            }
            futures::future::Either::Right(((), _)) => {
                debug!("The listener task is being shutdown")
            }
        }
        let _terminated = terminated.send(());
    };
    let _: JoinHandle<()> = tokio::spawn(task.instrument(debug_span!("Address", %address)));
    Ok(())
}

async fn process_listener(
    listener: TcpListener,
    notify: mpsc::Sender<PortForwardAcceptResponse>,
    streams: mpsc::Sender<TcpStream>,
) -> std::io::Result<()> {
    loop {
        let () = notify
            .send(PortForwardAcceptResponse {})
            .await
            .map_err(|error| {
                let message = format!("Failed to notify tcp_stream: {error}");
                std::io::Error::new(ErrorKind::BrokenPipe, message)
            })?;
        let (tcp_stream, _address) = listener.accept().await?;
        let () = streams.send(tcp_stream).await.map_err(|error| {
            let message = format!("Failed to register tcp_stream: {error}");
            std::io::Error::new(ErrorKind::BrokenPipe, message)
        })?;
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum BindLocalError {
    #[error("[{n}] Failed to resolve: {0}", n = self.name())]
    Hostname(std::io::Error),

    #[error("[{n}] Failed to bind: {0}", n = self.name())]
    Bind(std::io::Error),

    #[error("[{n}] The endpoint is already used: {0:?}", n = self.name())]
    EndpointInUse(EndpointId),
}

impl From<BindLocalError> for Status {
    fn from(mut error: BindLocalError) -> Self {
        let code = match &mut error {
            BindLocalError::Hostname { .. } => tonic::Code::InvalidArgument,
            BindLocalError::Bind { .. } => tonic::Code::InvalidArgument,
            BindLocalError::EndpointInUse { .. } => tonic::Code::AlreadyExists,
        };
        Self::new(code, error.to_string())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct BindRemoteError(Box<Status>);

impl From<BindRemoteError> for Status {
    fn from(BindRemoteError(mut status): BindRemoteError) -> Self {
        std::mem::replace(status.as_mut(), Status::ok(""))
    }
}

impl From<Status> for BindRemoteError {
    fn from(status: Status) -> Self {
        Self(Box::new(status))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct BindError(#[from] DistributedCallbackError<BindLocalError, BindRemoteError>);

impl From<BindError> for Status {
    fn from(BindError(error): BindError) -> Self {
        error.into()
    }
}
