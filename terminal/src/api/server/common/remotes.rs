use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::axum::Json;
use terrazzo::axum::Router;
use terrazzo::axum::routing::get;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::client_address::ClientAddress;
use crate::backend::auth::AuthConfig;
use crate::backend::auth::layer::AuthLayer;
use crate::backend::client_service::shared_service::remotes::list_remotes;
use crate::backend::config::DynConfig;

#[autoclone]
pub fn remotes_routes(
    config: &DiffArc<DynConfig>,
    auth_config: &DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
    server: &Arc<Server>,
) -> Router {
    let mesh = &config.mesh;
    let client_name = mesh.with(|mesh| Some(ClientName::from(mesh.as_ref()?.client_name.as_str())));
    Router::new()
        .route(
            "/remotes",
            get(|| {
                autoclone!(client_name, server);
                call_list_remotes(client_name, server)
            }),
        )
        .route_layer(AuthLayer {
            auth_config: auth_config.clone(),
        })
}

pub async fn call_list_remotes(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
) -> Json<Vec<ClientAddress>> {
    let my_client_name = my_client_name
        .map(|n| vec![n.to_string()])
        .unwrap_or_default();
    let mut remotes = list_remotes(&server, my_client_name).await;
    remotes.sort_by_key(|remote| remote.leaf());
    let remotes = remotes.into_iter().map(ClientAddress::from).collect();
    Json(remotes)
}
