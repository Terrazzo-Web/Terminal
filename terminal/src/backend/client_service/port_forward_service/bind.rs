use std::collections::hash_map;
use std::future::ready;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::net::ToSocketAddrs as _;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use futures::StreamExt as _;
use nameth::NamedEnumValues as _;
use nameth::NamedType as _;
use nameth::nameth;
use pin_project::pin_project;
use scopeguard::defer;
use terrazzo::autoclone;
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
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_common::handle::ServerHandle;
use trz_gateway_server::server::Server;

use crate::backend::client_service::port_forward_service::listeners::EndpointId;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_service_client::PortForwardServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress;

pub async fn dispatch(
    server: &Arc<Server>,
    mut requests: impl Stream<Item = Result<PortForwardEndpoint, Status>> + Unpin + Send + 'static,
) -> Result<BindStream, BindError> {
    let task = async move {
        debug!("Start");
        defer!(debug!("End"));
        let Some(first_message) = requests.next().await else {
            return Err(BindError::EmptyRequest);
        };
        let first_message =
            first_message.map_err(|status| BindError::RequestError(Box::new(status)))?;
        debug!("Port forward request: {first_message:?}");

        let remote = first_message.remote.clone().unwrap_or_default();
        let requests = requests.filter_map(|request| ready(request.ok()));
        let stream = BindCallback::process(server, &remote.via, (first_message, requests)).await?;
        return Ok(stream);
    };
    return task.instrument(info_span!("PortForward Bind")).await;
}

pub struct BindCallback<S: Stream<Item = PortForwardEndpoint>>(PhantomData<S>);

#[pin_project(project = BindStreamProj)]
pub enum BindStream {
    Local(#[pin] LocalBindStream),
    Remote(#[pin] RemoteBindStream),
}

#[pin_project]
pub struct LocalBindStream(
    #[pin] mpsc::Receiver<Result<PortForwardAcceptResponse, BindLocalError>>,
);

#[pin_project]
pub struct RemoteBindStream(#[pin] Streaming<PortForwardAcceptResponse>);

impl Stream for BindStream {
    type Item = Result<PortForwardAcceptResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            BindStreamProj::Local(local) => {
                match std::task::ready!(local.project().0.poll_recv(cx)) {
                    Some(Ok(response)) => Some(Ok(response)),
                    Some(Err(error)) => Some(Err(error.into())),
                    None => None,
                }
                .into()
            }
            BindStreamProj::Remote(remote) => remote.project().0.poll_next(cx),
        }
    }
}

impl<S: Stream<Item = PortForwardEndpoint> + Send + Unpin + 'static> DistributedCallback
    for BindCallback<S>
{
    type Request = (PortForwardEndpoint, S);
    type Response = BindStream;
    type LocalError = BindLocalError;
    type RemoteError = BindRemoteError;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        (endpoint, requests): (PortForwardEndpoint, S),
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
            let endpoint = PortForwardEndpoint {
                remote: Some(ClientAddress::of(client_address)),
                ..endpoint
            };
            let requests = futures::stream::once(ready(endpoint)).chain(requests);
            let mut client = PortForwardServiceClient::new(channel);
            let response = client.bind(requests).await?;
            Ok(BindStream::Remote(RemoteBindStream(response.into_inner())))
        }
        .instrument(info_span!("Remote"))
        .await
    }

    #[autoclone]
    async fn local(
        _server: &Arc<Server>,
        (endpoint, requests): (PortForwardEndpoint, S),
    ) -> Result<BindStream, BindLocalError> {
        let mut requests = futures::stream::once(ready(endpoint)).chain(requests);
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let (notify_tx, notify_rx) = mpsc::channel(3);
            let requests_task = async move {
                autoclone!(notify_tx);
                let mut next = requests.next().await;
                while let Some(request) = next.take() {
                    let span = debug_span!("Request", host = request.host, port = request.port);
                    next = async {
                        debug!("Start: port forward request = {request:?}");
                        defer!(debug!("End"));
                        let shutdown = process_request(&notify_tx, request).await?;
                        debug!("Waiting for next request");
                        let next = requests.next().await;
                        debug!("Shuting down listeners");
                        let () = shutdown.await;
                        return Ok::<_, BindLocalError>(next);
                    }
                    .instrument(span)
                    .await?;
                }
                Ok::<_, BindLocalError>(())
            };
            let requests_task = async move {
                match requests_task.await {
                    Ok(()) => (),
                    Err(error) => match notify_tx.send(Err(error)).await {
                        Ok(()) => (),
                        Err(error) => warn!("Failed to return error: {error}"),
                    },
                };
            };
            tokio::spawn(requests_task.in_current_span());
            Ok(BindStream::Local(LocalBindStream(notify_rx)))
        }
        .instrument(debug_span!("Local"))
        .await
    }
}

async fn process_request(
    notify: &mpsc::Sender<Result<PortForwardAcceptResponse, BindLocalError>>,
    endpoint: PortForwardEndpoint,
) -> Result<impl Future<Output = ()>, BindLocalError> {
    let endpoint_id = EndpointId {
        host: endpoint.host,
        port: endpoint.port,
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
            let (tx, rx) = oneshot::channel();
            let () = tx.send(streams_rx).expect("Failed to set streams");
            entry.insert(rx);
        }
    }

    for address in addresses {
        let (shutdown, terminated, handle) = ServerHandle::new(format!("port forward"));
        handles.push(handle);
        process_socket_address(
            address,
            notify.clone(),
            streams_tx.clone(),
            shutdown,
            terminated,
        )
        .await?;
    }

    let shutdown = async move {
        debug!("Removing streams");
        super::listeners::listeners().remove(&endpoint_id);
        for handle in handles {
            let () = handle
                .stop("PortForward request shutdown")
                .await
                .unwrap_or_else(|error| warn!("{error}"));
        }
        debug!("All listeners have shutdown");
    };

    Ok(shutdown)
}

async fn process_socket_address(
    address: SocketAddr,
    notify: mpsc::Sender<Result<PortForwardAcceptResponse, BindLocalError>>,
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
    let _: JoinHandle<()> = tokio::spawn(task.instrument(info_span!("Address", %address)));
    Ok(())
}

async fn process_listener(
    listener: TcpListener,
    notify: mpsc::Sender<Result<PortForwardAcceptResponse, BindLocalError>>,
    streams: mpsc::Sender<TcpStream>,
) -> std::io::Result<()> {
    info!("Listening start");
    defer!(info!("Listening end"));
    loop {
        let () = notify
            .send(Ok(PortForwardAcceptResponse {}))
            .await
            .map_err(|error| {
                let message = format!("Failed to notify tcp_stream: {error}");
                std::io::Error::new(ErrorKind::BrokenPipe, message)
            })?;
        let (tcp_stream, address) = listener.accept().await?;
        debug!("Received connection on {address}");
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
pub enum BindError {
    #[error("[{n}] Empty request", n = Self::type_name())]
    EmptyRequest,

    #[error("[{n}] Failed request: {0}", n = Self::type_name())]
    RequestError(Box<Status>),

    #[error("[{n}] {0}", n = Self::type_name())]
    Dispatch(#[from] DistributedCallbackError<BindLocalError, BindRemoteError>),
}

impl From<BindError> for Status {
    fn from(error: BindError) -> Self {
        let code = match error {
            BindError::EmptyRequest => tonic::Code::InvalidArgument,
            BindError::RequestError { .. } => tonic::Code::FailedPrecondition,
            BindError::Dispatch(error) => return error.into(),
        };
        Self::new(code, error.to_string())
    }
}
