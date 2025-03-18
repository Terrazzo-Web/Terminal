use std::sync::Arc;

use terrazzo::axum::Json;
use tracing::info_span;
use trz_gateway_server::server::Server;

use crate::api::client_id::ClientId;

pub async fn remotes(server: Arc<Server>) -> Json<Vec<ClientId>> {
    let _span = info_span!("Remotes").entered();
    let mut clients = server.connections().clients().collect::<Vec<_>>();
    clients.sort();
    clients.into()
}
