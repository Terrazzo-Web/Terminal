use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::http::StatusCode;
use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::NewIdRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes::next_terminal_id;

pub async fn new_id(
    server: &Server,
    client_address: &[impl AsRef<str>],
) -> Result<i32, NewIdError> {
    async {
        info!("Start");
        defer!(info!("Done"));
        match client_address {
            [rest @ .., client_address_leaf] => {
                let client_address_leaf = ClientName::from(client_address_leaf.as_ref());
                let channel = server
                    .connections()
                    .get_client(&client_address_leaf)
                    .ok_or_else(|| NewIdError::RemoteClientNotFound(client_address_leaf))?;
                Ok(ClientServiceClient::new(channel)
                    .new_id(NewIdRequest {
                        address: Some(ClientAddress {
                            via: rest.into_iter().map(|x| x.as_ref().to_owned()).collect(),
                        }),
                    })
                    .await
                    .map_err(NewIdError::RemoteClientError)?
                    .get_ref()
                    .next)
            }
            [] => Ok(next_terminal_id()),
        }
    }
    .instrument(info_span!("New ID"))
    .await
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
    #[error("[{n}] {0}", n = self.name())]
    RemoteClientError(tonic::Status),

    #[error("[{n}] Client not found: {0}", n = self.name())]
    RemoteClientNotFound(ClientName),
}

impl IsHttpError for NewIdError {
    fn status_code(&self) -> StatusCode {
        match self {
            NewIdError::RemoteClientError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NewIdError::RemoteClientNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}
