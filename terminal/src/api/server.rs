#![cfg(feature = "server")]

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
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::server::terminal_api::router::terminal_api_routes;
use crate::api::server::terminal_api::stream;
use crate::backend::auth::AuthConfig;
use crate::backend::auth::AuthLayer;
use crate::backend::config::DynConfig;

mod correlation_id;
mod remotes;
mod terminal_api;

#[autoclone]
pub fn api_routes(
    config: &DiffArc<DynConfig>,
    auth_config: &DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
    server: &Arc<Server>,
) -> Router {
    let mesh = &config.mesh;
    let client_name = mesh.with(|mesh| Some(ClientName::from(mesh.as_ref()?.client_name.as_str())));
    Router::new()
        .route(
            "/login",
            post(|cookies, headers, password| {
                autoclone!(config, auth_config);
                login(config, auth_config, cookies, headers, password)
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
        })
        .merge(terminal_api_routes(config, auth_config, server))
}

pub async fn login(
    config: DiffArc<DynConfig>,
    auth_config: DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
    cookies: CookieJar,
    headers: HeaderMap,
    Json(password): Json<Option<String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _span = info_span!("Login").entered();
    let server = config.server.get();
    let result = move || {
        match (&server.password, &password) {
            (None, _) => debug!("Password not required"),
            (Some(_), None) => {
                debug!("Password not provided, checking token");
                let _ = auth_config.with(|auth_config| auth_config.validate(&headers))?;
            }
            (Some(_), Some(password)) => {
                debug!("Password provided, verify password");
                let () = server
                    .verify_password(password)
                    .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;
            }
        }

        let token = auth_config
            .with(|auth_config| auth_config.make_token())
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
        Ok((cookies.add(token), "OK"))
    };
    return result()
        .inspect(|(_cookies, result)| info!("{result}"))
        .inspect_err(|(status_code, error)| warn!("Failed: {status_code} {error}"));
}
