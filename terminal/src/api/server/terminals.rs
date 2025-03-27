use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_server::server::Server;

use crate::api::TerminalDef;
use crate::backend::client_service::terminals::list_terminals;

pub async fn list(server: Arc<Server>) -> Json<Vec<TerminalDef>> {
    Json(
        list_terminals(&server, &[])
            .await
            .into_iter()
            .map(TerminalDef::from)
            .collect(),
    )
}
