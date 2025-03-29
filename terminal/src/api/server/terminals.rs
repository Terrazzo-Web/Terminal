use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::TerminalDef;
use crate::backend::client_service::terminals::list_terminals;

pub async fn list(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
) -> Json<Vec<TerminalDef>> {
    let my_client_name = my_client_name
        .map(|n| vec![n.to_string()])
        .unwrap_or_default();
    let mut terminals: Vec<_> = list_terminals(&server, &my_client_name)
        .await
        .into_iter()
        .map(TerminalDef::from)
        .collect();
    terminals.sort_by_key(|terminal| terminal.order);
    Json(terminals)
}
