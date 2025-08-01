use std::sync::Arc;

use futures::TryFutureExt as _;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::warn;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

pub trait DistributedCallback {
    type Request;
    type Response;
    type LocalError: std::error::Error;
    type RemoteError: std::error::Error;

    fn process(
        server: &Arc<Server>,
        client_address: &[impl AsRef<str>],
        request: Self::Request,
    ) -> impl Future<
        Output = Result<
            Self::Response,
            DistributedCallbackError<Self::LocalError, Self::RemoteError>,
        >,
    > {
        async move {
            match client_address {
                [rest @ .., client_address_leaf] => {
                    let client_address_leaf = ClientName::from(client_address_leaf.as_ref());
                    let channel = server
                        .connections()
                        .get_client(&client_address_leaf)
                        .ok_or_else(|| {
                            DistributedCallbackError::RemoteClientNotFound(client_address_leaf)
                        })?;
                    Ok(Self::remote(channel, rest, request)
                        .await
                        .map_err(DistributedCallbackError::RemoteError)?)
                }
                [] => Ok(Self::local(server, request)
                    .await
                    .map_err(DistributedCallbackError::LocalError)?),
            }
        }
        .inspect_err(|error| warn!("Failed: {error}"))
    }

    fn local(
        server: &Arc<Server>,
        request: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, Self::LocalError>>;

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        request: Self::Request,
    ) -> Result<Self::Response, Self::RemoteError>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send;
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DistributedCallbackError<L: std::error::Error, R: std::error::Error> {
    #[error("[{n}] {0}", n = self.name())]
    RemoteError(R),

    #[error("[{n}] {0}", n = self.name())]
    LocalError(L),

    #[error("[{n}] Client not found: {0}", n = self.name())]
    RemoteClientNotFound(ClientName),
}

impl<L: IsHttpError, R: IsHttpError> IsHttpError for DistributedCallbackError<L, R> {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::RemoteError(error) => error.status_code(),
            Self::LocalError(error) => error.status_code(),
            Self::RemoteClientNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}

impl<L: std::error::Error + Into<Status>, R: std::error::Error + Into<Status>>
    From<DistributedCallbackError<L, R>> for Status
{
    fn from(error: DistributedCallbackError<L, R>) -> Self {
        match error {
            DistributedCallbackError::RemoteError(error) => error.into(),
            DistributedCallbackError::LocalError(error) => error.into(),
            error @ DistributedCallbackError::RemoteClientNotFound { .. } => {
                Status::not_found(error.to_string())
            }
        }
    }
}

impl<L: std::error::Error, R: std::error::Error> DistributedCallbackError<L, R> {
    pub fn map_local<LL: std::error::Error>(
        self,
        f: impl FnOnce(L) -> LL,
    ) -> DistributedCallbackError<LL, R> {
        match self {
            Self::RemoteError(error) => DistributedCallbackError::RemoteError(error),
            Self::LocalError(error) => DistributedCallbackError::LocalError(f(error)),
            Self::RemoteClientNotFound(client_name) => {
                DistributedCallbackError::RemoteClientNotFound(client_name)
            }
        }
    }

    pub fn map_remote<RR: std::error::Error>(
        self,
        f: impl FnOnce(R) -> RR,
    ) -> DistributedCallbackError<L, RR> {
        match self {
            Self::RemoteError(error) => DistributedCallbackError::RemoteError(f(error)),
            Self::LocalError(error) => DistributedCallbackError::LocalError(error),
            Self::RemoteClientNotFound(client_name) => {
                DistributedCallbackError::RemoteClientNotFound(client_name)
            }
        }
    }
}
