use axum_extra::extract::CookieJar;
use terrazzo::autoclone;
use terrazzo::axum::Json;
use terrazzo::axum::Router;
use terrazzo::axum::response::IntoResponse;
use terrazzo::axum::routing::post;
use terrazzo::http::HeaderMap;
use terrazzo::http::StatusCode;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;

use crate::backend::auth::AuthConfig;
use crate::backend::config::DynConfig;

#[autoclone]
pub fn login_routes(
    config: &DiffArc<DynConfig>,
    auth_config: &DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
) -> Router {
    Router::new().route(
        "/login",
        post(|cookies, headers, password| {
            autoclone!(config, auth_config);
            login(config, auth_config, cookies, headers, password)
        }),
    )
}

async fn login(
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
