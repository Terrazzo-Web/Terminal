use std::collections::hash_map;
use std::future::ready;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::ToSocketAddrs as _;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use futures::StreamExt;
use nameth::NamedEnumValues as _;
use nameth::nameth;
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
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptResponse;
use crate::backend::protos::terrazzo::portforward::port_forward_service_client::PortForwardServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress;

pub enum BindStream {
    Local(LocalBindStream),
    Remote(RemoteBindStream),
}

pub struct LocalBindStream(mpsc::Receiver<()>);

pub struct RemoteBindStream(Streaming<PortForwardAcceptResponse>);

impl Stream for BindStream {
    type Item = Result<PortForwardAcceptResponse, Status>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

impl DistributedCallback for BindStream {
    type Request = Streaming<PortForwardAcceptRequest>;
    type Response = BindStream;
    type LocalError = PortForwardError;
    type RemoteError = Status;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        mut requests: Streaming<PortForwardAcceptRequest>,
    ) -> Result<BindStream, Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let mut client = PortForwardServiceClient::new(channel);
            let Some(first_request) = requests.next().await else {
                return Err(Status::invalid_argument("Empty request stream"));
            };
            let first_request = first_request?;
            debug!("Port forward request: {first_request:?}");

            let requests = futures::stream::once(ready(PortForwardAcceptRequest {
                remote: Some(ClientAddress::of(client_address)),
                ..first_request
            }))
            .chain(requests.filter_map(|request| ready(request.ok())));

            let response = client.bind(requests).await?;
            let response = response.into_inner();
            Ok(BindStream::Remote(RemoteBindStream(response)).into())
        }
        .instrument(debug_span!("PortForward remote"))
        .await
    }

    async fn local(
        _server: &Server,
        mut requests: Streaming<PortForwardAcceptRequest>,
    ) -> Result<BindStream, PortForwardError> {
        async move {
            debug!("Start");
            defer!(debug!("End"));
            let mut next = requests.next().await;
            let (notify_tx, notify_rx) = mpsc::channel(3);
            while let Some(request) = next.take() {
                debug!("Port forward request: {request:?}");
                defer!(debug!("Port forward shutdown"));
                let request =
                    request.map_err(|error| PortForwardError::RequestFailed(Box::new(error)))?;
                let span = debug_span!("Request", host = request.host, port = request.port);
                let shutdown = process_request(&notify_tx, request)
                    .instrument(span)
                    .await?;
                next = requests.next().await;
                let () = shutdown.await;
            }
            Ok(BindStream::Local(LocalBindStream(notify_rx)))
        }
        .instrument(debug_span!("PortForward"))
        .await
    }
}

async fn process_request(
    notify_tx: &mpsc::Sender<()>,
    request: PortForwardAcceptRequest,
) -> Result<impl Future<Output = ()>, PortForwardError> {
    let endpoint_id = EndpointId {
        host: request.host,
        port: request.port,
    };
    let addresses = format!("{}:{}", endpoint_id.host, endpoint_id.port)
        .to_socket_addrs()
        .map_err(PortForwardError::Hostname)?;

    let mut handles = vec![];
    let (streams_tx, streams_rx) = mpsc::channel(3);
    match super::listeners::listeners().entry(endpoint_id.clone()) {
        hash_map::Entry::Occupied(_occupied) => {
            return Err(PortForwardError::EndpointInUse(endpoint_id));
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
    notify: mpsc::Sender<()>,
    streams: mpsc::Sender<TcpStream>,
    shutdown: impl Future<Output = ()> + Send + 'static,
    terminated: oneshot::Sender<()>,
) -> Result<(), PortForwardError> {
    let listener = TcpListener::bind(address)
        .await
        .map_err(PortForwardError::Bind)?;
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
    notify: mpsc::Sender<()>,
    streams: mpsc::Sender<TcpStream>,
) -> std::io::Result<()> {
    loop {
        let () = notify.send(()).await.map_err(|error| {
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
pub enum PortForwardError {
    #[error("[{n}] Failed to resolve: {0}", n = self.name())]
    Hostname(std::io::Error),

    #[error("[{n}] Failed to bind: {0}", n = self.name())]
    Bind(std::io::Error),

    #[error("[{n}] Request failed: {0}", n = self.name())]
    RequestFailed(Box<Status>),

    #[error("[{n}] The endpoint is already used: {0:?}", n = self.name())]
    EndpointInUse(EndpointId),
}

impl From<PortForwardError> for tonic::Status {
    fn from(mut error: PortForwardError) -> Self {
        let code = match &mut error {
            PortForwardError::Hostname { .. } => tonic::Code::InvalidArgument,
            PortForwardError::Bind { .. } => tonic::Code::InvalidArgument,
            PortForwardError::RequestFailed(status) => {
                return std::mem::replace(status.as_mut(), Status::ok(""));
            }
            PortForwardError::EndpointInUse { .. } => tonic::Code::AlreadyExists,
        };
        Self::new(code, error.to_string())
    }
}
