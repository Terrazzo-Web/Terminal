#![cfg(feature = "server")]

use std::sync::Arc;

use terrazzo::axum::Router;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;
use trz_gateway_server::server::Server;

use crate::api::server::common::login::login_routes;
use crate::api::server::common::remotes::remotes_routes;
use crate::backend::auth::AuthConfig;
use crate::backend::config::DynConfig;

mod common;
mod correlation_id;

mod terminal_api;

pub fn api_routes(
    config: &DiffArc<DynConfig>,
    auth_config: &DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
    server: &Arc<Server>,
) -> Router {
    let router = Router::new()
        .merge(login_routes(config, auth_config))
        .merge(remotes_routes(config, auth_config, server));

    #[cfg(feature = "terminal")]
    let router = router.merge(terminal_api::router::terminal_api_routes(
        config,
        auth_config,
        server,
    ));

    return router;
}
