use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::axum::Router;
use terrazzo::axum::routing::get;
use terrazzo::axum::routing::post;
use tower_http::validate_request::ValidateRequestHeaderLayer;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

mod auth;
mod correlation_id;
mod new_id;
mod remotes;
mod resize;
mod set_order;
mod set_title;
mod stream;
mod terminals;
mod write;

pub use auth::AuthConfig;

#[autoclone]
pub fn route(
    client_name: &Option<ClientName>,
    server: &Arc<Server>,
    auth_config: &Arc<AuthConfig>,
) -> Router {
    let client_name = client_name.clone();
    let server = server.clone();
    Router::new()
        .route(
            "/terminals",
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
        .route("/stream/pipe", post(stream::pipe))
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
        .route(
            "/remotes",
            get(|| {
                autoclone!(client_name, server);
                remotes::list(client_name, server)
            }),
        )
        .route_layer(ValidateRequestHeaderLayer::custom(auth::validate(
            auth_config.clone(),
        )))
}
