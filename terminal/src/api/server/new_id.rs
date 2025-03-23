use std::sync::Arc;
use std::vec;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::axum::extract::Json;
use terrazzo::http::StatusCode;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;
use uuid::Uuid;

use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::backend::protos::terrazzo::gateway::client::NewIdRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes::next_terminal_id;

pub async fn new_id(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    Json(client_name): Json<Option<ClientName>>,
) -> Result<Json<TerminalDef>, HttpError<NewIdError>> {
    let next = match &client_name {
        Some(client_name) => {
            let channel = server
                .connections()
                .get_client(client_name)
                .ok_or_else(|| NewIdError::RemoteClientNotFound(client_name.to_owned()))?;
            ClientServiceClient::new(channel)
                .new_id(NewIdRequest {})
                .await
                .map_err(NewIdError::RemoteClientError)?
                .get_ref()
                .next
        }
        None => next_terminal_id(),
    };
    let title = if let Some(my_client_name) = &my_client_name {
        format!("Terminal {my_client_name}:{next}")
    } else {
        format!("Terminal {next}")
    };
    let id = if cfg!(feature = "concise_traces") {
        Uuid::new_v4().to_string()
    } else if let Some(my_client_name) = &my_client_name {
        format!("T-{my_client_name}-{next}")
    } else {
        format!("T-{next}")
    }
    .into();
    Ok(Json(TerminalDef {
        id,
        title: TabTitle {
            shell_title: title,
            override_title: None,
        },
        order: next,
        via: vec![],
    }))
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
