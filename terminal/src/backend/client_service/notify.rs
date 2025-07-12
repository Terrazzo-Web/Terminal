use std::future::ready;

use futures::SinkExt as _;
use futures::StreamExt as _;
use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Code;
use tonic::Status;
use tonic::codegen::StdError;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use self::request::remote::RemoteRequestStream;
use self::response::HybridResponseStream;
use super::notify::request::HybridRequestStream;
use super::remote_fn::RemoteFnError;
use super::remote_fn::remote_fn_server;
use crate::backend::client_service::notify::request::local::LocalRequestStream;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::NotifyRequest as NotifyRequestProto;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::backend::protos::terrazzo::gateway::client::notify_request::RequestType as RequestTypeProto;
use crate::text_editor::notify::NotifyRequest;
use crate::text_editor::notify::service::notify as notify_local;

mod request;
mod response;

pub use self::response::remote::RemoteResponseStream;

pub fn notify_hybrid(request: HybridRequestStream) -> Result<HybridResponseStream, NotifyError> {
    let response_stream = async {
        debug!("Start");
        defer!(debug!("Done"));
        let server = remote_fn_server()?;
        let mut request = LocalRequestStream(request);
        if let Some(next) = request.next().await {
            let next = match next {
                Ok(next) => {
                    debug!("Next: {:?}", next);
                    next
                }
                Err(error) => return Err(NotifyError::InvalidStart(error)),
            };
            match next {
                NotifyRequest::Start { remote } => {
                    let response = if remote.is_empty() {
                        let request = HybridRequestStream::Local(
                            futures::stream::once(ready(Ok(NotifyRequest::Start {
                                remote: Default::default(),
                            })))
                            .chain(request)
                            .into(),
                        );
                        NotifyCallback::process(&server, &remote, request)
                    } else {
                        NotifyCallback::process(&server, &remote, request.0)
                    };
                    return response.await.map_err(NotifyError::Error);
                }
                NotifyRequest::Watch { .. } | NotifyRequest::UnWatch { .. } => {
                    return Err(NotifyError::WatchBeforeStart);
                }
            }
        }
        return Err(NotifyError::MissingStart);
    };
    let (mut tx, rx) = mpsc::unbounded();
    let response = async move {
        let response_stream = match response_stream.await {
            Ok(response_stream) => response_stream,
            Err(error) => {
                if let Err(mpsc::SendError { .. }) = tx.send(Err(error.into())).await {
                    warn!("Stream closed");
                }
                return;
            }
        };
        let mut response_stream = BoxedStream::from(response_stream);
        while let Some(next) = response_stream.next().await {
            if let Err(mpsc::SendError { .. }) = tx.send(next).await {
                warn!("Stream closed");
                return;
            }
        }
    };
    tokio::spawn(response.instrument(debug_span!("NotifyHybrid")));
    return Ok(HybridResponseStream::Local(BoxedStream::from(rx)));
}

struct NotifyCallback;

impl DistributedCallback for NotifyCallback {
    type Request = HybridRequestStream;
    type Response = HybridResponseStream;
    type LocalError = NotifyErrorImpl;
    type RemoteError = Box<Status>;

    async fn local(
        _server: &Server,
        request: HybridRequestStream,
    ) -> Result<HybridResponseStream, NotifyErrorImpl> {
        notify_local(request.into())
            .map_err(NotifyErrorImpl::Local)
            .map(HybridResponseStream::Local)
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        request: HybridRequestStream,
    ) -> Result<HybridResponseStream, Box<Status>>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: tonic::transport::Body<Data = server_fn::Bytes> + Send + 'static,
        <T::ResponseBody as tonic::transport::Body>::Error: Into<StdError> + Send,
    {
        let client_address = ClientAddressProto::of(client_address);
        let request = RemoteRequestStream(request).filter_map(|request| ready(request.ok()));
        let request = futures::stream::once(ready(NotifyRequestProto {
            request_type: Some(RequestTypeProto::Address(client_address)),
        }))
        .chain(request);
        let response = client.notify(request).await?.into_inner();
        Ok(HybridResponseStream::Remote(Box::new(response)))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NotifyError {
    #[error("[{n}] {0}", n = self.name())]
    Error(DistributedCallbackError<NotifyErrorImpl, Box<Status>>),

    #[error("[{n}] {0}", n = self.name())]
    InvalidStart(ServerFnError),

    #[error("[{n}] Can't Watch/UnWatch before Start message", n = self.name())]
    WatchBeforeStart,

    #[error("[{n}] Empty essage doesn't have a RequestType", n = self.name())]
    MissingRequestType,

    #[error("[{n}] Missing Start message", n = self.name())]
    MissingStart,

    #[error("[{n}] {0}", n = self.name())]
    RemoteFnError(#[from] RemoteFnError),
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NotifyErrorImpl {
    #[error("[{n}] {0}", n = self.name())]
    Local(ServerFnError),
}

impl From<NotifyError> for Status {
    fn from(mut error: NotifyError) -> Self {
        let code = match &mut error {
            NotifyError::Error(DistributedCallbackError::RemoteError(error)) => {
                return std::mem::replace(error, Status::ok(""));
            }
            NotifyError::Error(DistributedCallbackError::LocalError { .. })
            | NotifyError::RemoteFnError { .. } => Code::Internal,
            NotifyError::Error(DistributedCallbackError::RemoteClientNotFound { .. })
            | NotifyError::InvalidStart { .. }
            | NotifyError::WatchBeforeStart
            | NotifyError::MissingRequestType
            | NotifyError::MissingStart => Code::InvalidArgument,
        };
        return Status::new(code, error.to_string());
    }
}
