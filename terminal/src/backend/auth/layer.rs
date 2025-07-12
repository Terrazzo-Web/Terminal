use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::SystemTime;

use axum_extra::extract::CookieJar;
use jsonwebtoken::TokenData;
use terrazzo::axum::body::Body;
use terrazzo::axum::response::IntoResponse as _;
use terrazzo::http::Request;
use terrazzo::http::Response;
use tower::Layer;
use tower::Service;
use tracing::debug;
use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode;

use crate::backend::auth::AuthConfig;
use crate::backend::auth::Claims;

#[derive(Clone)]
pub struct AuthLayer {
    pub auth_config: DiffArc<DynamicConfig<DiffArc<AuthConfig>, mode::RO>>,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            layer: self.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    layer: AuthLayer,
    inner: S,
}

impl<S> Service<Request<Body>> for AuthService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let auth_config = self.layer.auth_config.clone();
        Box::pin(async move {
            let token_data =
                match auth_config.with(|auth_config| auth_config.validate(request.headers())) {
                    Ok(token_data) => token_data,
                    Err(error) => return Ok(error.into_response()),
                };

            let response = inner.call(request).await?;
            return Ok(refresh_auth_token(&auth_config, token_data, response));
        })
    }
}

fn refresh_auth_token(
    auth_config: &DynamicConfig<DiffArc<AuthConfig>, mode::RO>,
    token_data: TokenData<Claims>,
    response: Response<Body>,
) -> Response<Body> {
    let Ok(expiration) = token_data.claims.exp.duration_since(SystemTime::now()) else {
        return response;
    };
    let token_refresh = auth_config.with(|auth_config| auth_config.token_refresh);
    if expiration > token_refresh {
        debug!("The auth cookie expires in {expiration:?} > {token_refresh:?}");
        return response;
    }

    let Ok(token) = auth_config
        .with(|auth_config| auth_config.make_token())
        .inspect_err(|error| warn!("Failed to create refreshed token: {error}"))
    else {
        return response;
    };

    debug!("Issued a new token");
    let cookies = CookieJar::from_headers(response.headers()).add(token);
    return (cookies, response).into_response();
}
