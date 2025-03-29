use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remotes::list_remotes;

pub async fn list(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
) -> Json<Vec<ClientAddress>> {
    let my_client_name = my_client_name
        .map(|n| vec![n.to_string()])
        .unwrap_or_default();
    let mut remotes = list_remotes(&server, &my_client_name).await;
    remotes.sort_by_key(|remote| remote.leaf());
    let remotes = remotes.into_iter().map(ClientAddress::from).collect();
    Json(remotes)
}
