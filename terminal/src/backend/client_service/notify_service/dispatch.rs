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
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use tracing::warn;

use crate::backend::client_service::notify_service::callback::NotifyCallback;
use crate::backend::client_service::notify_service::callback::NotifyLocalError;
use crate::backend::client_service::notify_service::request::HybridRequestStream;
use crate::backend::client_service::notify_service::request::local::LocalRequestStream;
use crate::backend::client_service::notify_service::response::HybridResponseStream;
use crate::backend::client_service::remote_fn::RemoteFnError;
use crate::backend::client_service::remote_fn::remote_fn_server;
use crate::backend::client_service::routing::DistributedCallback as _;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::text_editor::notify::NotifyRequest;

/// Dispatches the Notify request either locally or through the gRPC tunnel
pub fn notify_dispatch(request: HybridRequestStream) -> Result<HybridResponseStream, NotifyError> {
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NotifyError {
    #[error("[{n}] {0}", n = self.name())]
    Error(DistributedCallbackError<NotifyLocalError, Box<Status>>),

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
