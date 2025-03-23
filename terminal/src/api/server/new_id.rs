use std::sync::Arc;

use terrazzo::axum::extract::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;
use uuid::Uuid;

use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::api::client_address::ClientAddress;
use crate::backend::client_service::new_id;
use crate::backend::client_service::new_id::NewIdError;

pub async fn new_id(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    Json(client_address): Json<ClientAddress>,
) -> Result<Json<TerminalDef>, HttpError<NewIdError>> {
    let next = new_id::new_id(&server, client_address.as_slice()).await?;
    let client_name = client_address.last().or(my_client_name.as_ref());

    let title = if let Some(client_name) = client_name {
        format!("Terminal {client_name}:{next}")
    } else {
        format!("Terminal {next}")
    };

    let id = if cfg!(feature = "concise_traces") {
        Uuid::new_v4().to_string()
    } else if let Some(client_name) = client_name {
        format!("T-{client_name}-{next}")
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
        via: client_address,
    }))
}
