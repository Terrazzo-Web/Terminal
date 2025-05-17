use std::sync::Arc;

use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use http::HeaderMap;
use http::StatusCode;
use terrazzo::autoclone;
use terrazzo::axum;
use terrazzo::axum::Json;
use terrazzo::http;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

mod correlation_id;
mod new_id;
mod remotes;
mod resize;
mod set_order;
mod set_title;
mod stream;
mod terminals;
mod write;

use crate::backend::auth::AuthConfig;
use crate::backend::auth::AuthLayer;
use crate::backend::config_file::ConfigFile;

#[autoclone]
pub fn api_routes(
    client_name: &Option<ClientName>,
    auth_config: &Arc<AuthConfig>,
    config_file: &Arc<ConfigFile>,
    server: &Arc<Server>,
) -> Router {
    let client_name = client_name.clone();
    let server = server.clone();
    Router::new()
        .route(
            "/login",
            post(|cookies, headers, password| {
                autoclone!(auth_config, config_file);
                login(auth_config, config_file, cookies, headers, password)
            }),
        )
        .nest(
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
                .route_layer(AuthLayer {
                    auth_config: auth_config.clone(),
                }),
        )
}

pub async fn login(
    auth_config: Arc<AuthConfig>,
    config_file: Arc<ConfigFile>,
    cookies: CookieJar,
    headers: HeaderMap,
    Json(password): Json<Option<String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Some(password) = &password else {
        auth_config.validate(&headers)?;
        return Ok((cookies, "OK"));
    };

    if config_file.server.password.is_some() {
        let () = config_file
            .server
            .verify_password(&password)
            .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
    }
    let token = auth_config
        .make_token()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((cookies.add(token), "OK"))
}
