use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::TerminalDef;
use crate::backend::client_service::terminal_service;

pub async fn list(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
) -> Json<Vec<TerminalDef>> {
    let my_client_name = my_client_name
        .map(|n| vec![n.to_string()])
        .unwrap_or_default();
    let terminals = self::terminal_service::list::list_terminals(&server, my_client_name).await;
    let mut terminals: Vec<_> = terminals.into_iter().map(TerminalDef::from).collect();
    terminals.sort_by_key(|terminal| terminal.order);
    Json(terminals)
}
