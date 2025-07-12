use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use axum::routing::post;
use terrazzo::autoclone;
use terrazzo::axum;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::server::terminal_api::new_id;
use crate::api::server::terminal_api::resize;
use crate::api::server::terminal_api::set_order;
use crate::api::server::terminal_api::set_title;
use crate::api::server::terminal_api::stream;
use crate::api::server::terminal_api::terminals;
use crate::api::server::terminal_api::write;
use crate::backend::auth::AuthConfig;
use crate::backend::auth::AuthLayer;
use crate::backend::config::DynConfig;

#[autoclone]
pub fn terminal_api_routes(
    config: &DiffArc<DynConfig>,
    auth_config: &DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
    server: &Arc<Server>,
) -> Router {
    let mesh = &config.mesh;
    let client_name = mesh.with(|mesh| Some(ClientName::from(mesh.as_ref()?.client_name.as_str())));
    let server = server.clone();

    Router::new().nest(
        "/terminal",
        Router::new()
            .route(
                "/list",
                get(|| {
                    autoclone!(client_name, server);
                    terminals::list(client_name, server)
                }),
            )
            .route(
                "/new_id",
                post(move |request| {
                    autoclone!(client_name, server);
                    new_id::new_id(client_name, server, request)
                }),
            )
            .route(
                "/stream/ack",
                post(|request| {
                    autoclone!(server);
                    stream::ack(server, request)
                }),
            )
            .route(
                "/stream/close",
                post(|request| {
                    autoclone!(server);
                    stream::close(server, request)
                }),
            )
            .route(
                "/stream/pipe",
                post(|correlation_id| {
                    autoclone!(server);
                    stream::pipe(server, correlation_id)
                }),
            )
            .route("/stream/pipe/close", post(stream::close_pipe))
            .route("/stream/pipe/keepalive", post(stream::keepalive))
            .route(
                "/stream/register",
                post(|request| {
                    autoclone!(client_name, server);
                    stream::register(client_name, server, request)
                }),
            )
            .route(
                "/resize",
                post(|request| {
                    autoclone!(server);
                    resize::resize(server, request)
                }),
            )
            .route(
                "/set_title",
                post(|request| {
                    autoclone!(server);
                    set_title::set_title(server, request)
                }),
            )
            .route(
                "/set_order",
                post(|request| {
                    autoclone!(server);
                    set_order::set_order(server, request)
                }),
            )
            .route(
                "/write",
                post(|request| {
                    autoclone!(server);
                    write::write(server, request)
                }),
            )
            .route_layer(AuthLayer {
                auth_config: auth_config.clone(),
            }),
    )
}
