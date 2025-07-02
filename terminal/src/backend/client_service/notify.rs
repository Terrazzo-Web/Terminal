use std::future::ready;

use futures::StreamExt as _;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use server_fn::ServerFnError;
use tonic::Code;
use tonic::Status;
use tonic::codegen::StdError;
use trz_gateway_server::server::Server;

use self::request::remote::RemoteRequestStream;
use self::response::HybridResponseStream;
use super::routing::DistributedCallback;
use crate::backend::client_service::notify::request::HybridRequestStream;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::NotifyRequest as NotifyRequestProto;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::backend::protos::terrazzo::gateway::client::notify_request::RequestType as RequestTypeProto;
use crate::text_editor::notify::service::notify as notify_local;

mod request;
mod response;

pub use self::response::remote::RemoteResponseStream;

pub async fn notify_hybrid(
    server: &Server,
    request: HybridRequestStream,
) -> Result<HybridResponseStream, NotifyError> {
    let mut request = RemoteRequestStream(request);
    while let Some(next) = request.next().await {
        let next = match next {
            Ok(next) => next,
            Err(error) => return Err(NotifyError::InvalidStart(error)),
        };
        match next.request_type {
            Some(RequestTypeProto::Address(remote)) => {
                return Ok(NotifyCallback::process(server, &remote.via, request.0)
                    .await
                    .map_err(NotifyError::Error)?);
            }
            Some(RequestTypeProto::Watch { .. } | RequestTypeProto::Unwatch { .. }) => {
                return Err(NotifyError::WatchBeforeStart);
            }
            None => return Err(NotifyError::MissingRequestType),
        }
    }
    return Err(NotifyError::MissingStart);
}

struct NotifyCallback;

impl DistributedCallback for NotifyCallback {
    type Request = HybridRequestStream;
    type Response = HybridResponseStream;
    type LocalError = NotifyErrorImpl;
    type RemoteError = Status;

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
    ) -> Result<HybridResponseStream, Status>
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
        Ok(HybridResponseStream::Remote(response))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NotifyError {
    #[error("[{n}] {0}", n = self.name())]
    Error(DistributedCallbackError<NotifyErrorImpl, Status>),

    #[error("[{n}] {0}", n = self.name())]
    InvalidStart(Status),

    #[error("[{n}] Can't Watch/UnWatch before Start message", n = self.name())]
    WatchBeforeStart,

    #[error("[{n}] Empty essage doesn't have a RequestType", n = self.name())]
    MissingRequestType,

    #[error("[{n}] Missing Start message", n = self.name())]
    MissingStart,
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
            NotifyError::Error(DistributedCallbackError::LocalError { .. }) => Code::Internal,
            NotifyError::Error(DistributedCallbackError::RemoteClientNotFound { .. })
            | NotifyError::InvalidStart { .. }
            | NotifyError::WatchBeforeStart
            | NotifyError::MissingRequestType
            | NotifyError::MissingStart => Code::InvalidArgument,
        };
        return Status::new(code, error.to_string());
    }
}
