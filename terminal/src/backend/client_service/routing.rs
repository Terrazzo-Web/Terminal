#![allow(unused)]

use bytes::Bytes;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::Body;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

pub trait DistributedCallback {
    type Request;
    type Response;
    type LocalError: IsHttpError;
    type RemoteError: IsHttpError;

    fn process(
        server: &Server,
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
                    let client = ClientServiceClient::new(channel);
                    Ok(Self::remote(client, rest, request)
                        .await
                        .map_err(DistributedCallbackError::RemoteError)?)
                }
                [] => Ok(Self::local(request)
                    .await
                    .map_err(DistributedCallbackError::LocalError)?),
            }
        }
    }

    fn local(
        request: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, Self::LocalError>>;
    fn remote<T>(
        client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        request: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, Self::RemoteError>>
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
            DistributedCallbackError::RemoteError(error) => error.status_code(),
            DistributedCallbackError::LocalError(error) => error.status_code(),
            DistributedCallbackError::RemoteClientNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
