use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remotes::list_remotes;

pub async fn list(server: Arc<Server>) -> Json<Vec<ClientAddress>> {
    let mut remotes = list_remotes(&server, &[]).await;
    remotes.sort_by_key(|remote| remote.leaf());
    let remotes = remotes
        .into_iter()
        .map(|remote| {
            ClientAddress::from(
                remote
                    .via
                    .into_iter()
                    .map(ClientName::from)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    Json(remotes)
}
