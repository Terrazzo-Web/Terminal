use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_server::server::Server;

use crate::api::TerminalDef;
use crate::backend::client_service::terminals::list_terminals;

pub async fn list(server: Arc<Server>) -> Json<Vec<TerminalDef>> {
    let mut terminals: Vec<_> = list_terminals(&server, &[])
        .await
        .into_iter()
        .map(TerminalDef::from)
        .collect();
    terminals.sort_by_key(|terminal| terminal.order);
    Json(terminals)
}
