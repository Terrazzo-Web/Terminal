use std::sync::Arc;

use terrazzo::axum::extract::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;
use uuid::Uuid;

use crate::api::client_address::ClientAddress;
use crate::api::shared::terminal_schema::TabTitle;
use crate::api::shared::terminal_schema::TerminalAddress;
use crate::api::shared::terminal_schema::TerminalDef;
use crate::backend::client_service::terminal_service;

pub async fn new_id(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    Json(client_address): Json<ClientAddress>,
) -> Result<Json<TerminalDef>, HttpError<self::terminal_service::new_id::NewIdError>> {
    let next = self::terminal_service::new_id::new_id(&server, &client_address).await?;
    let client_name = client_address.last().or(my_client_name.as_ref());

    let title = if let Some(client_name) = client_name {
        format!("Terminal {client_name}:{next}")
    } else {
        format!("Terminal {next}")
    };

    let id = if cfg!(feature = "concise-traces") {
        Uuid::new_v4().to_string()
    } else if let Some(client_name) = client_name {
        format!("T-{client_name}-{next}")
    } else {
        format!("T-{next}")
    }
    .into();
    Ok(Json(TerminalDef {
        address: TerminalAddress {
            id,
            via: client_address,
        },
        title: TabTitle {
            shell_title: title,
            override_title: None,
        },
        order: next,
    }))
}
