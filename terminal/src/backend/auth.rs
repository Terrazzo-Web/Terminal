use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::SystemTime;

use axum::body::Body;
use axum::response::IntoResponse as _;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::cookie::SameSite;
use http::Request;
use http::Response;
use http::StatusCode;
use http::header::AUTHORIZATION;
use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use terrazzo::axum;
use terrazzo::http;
use terrazzo::http::HeaderMap;
use tower::Layer;
use tower::Service;
use tracing::debug;
use tracing::warn;
use uuid::Uuid;

use self::jwt_timestamp::Timestamp;
use super::config_file::ConfigFile;

mod jwt_timestamp;

pub static TOKEN_COOKIE_NAME: &str = "slt";

/// Original expiration of the cookie.
pub static DEFAULT_TOKEN_LIFETIME: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(300)
} else {
    Duration::from_secs(3600 * 24)
};

/// Refresh if the cookie has started to expire
pub static DEFAULT_TOKEN_REFRESH: Duration = if cfg!(debug_assertions) {
    DEFAULT_TOKEN_LIFETIME.saturating_sub(Duration::from_secs(10))
} else {
    DEFAULT_TOKEN_LIFETIME.saturating_sub(Duration::from_secs(3600))
};

pub struct AuthConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    token_cookie_lifetime: Duration,
    token_cookie_refresh: Duration,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct Claims<T = Timestamp> {
    exp: T,
    nbf: T,
}

impl AuthConfig {
    pub fn new(config_file: &ConfigFile) -> Self {
        Self {
            token_cookie_lifetime: config_file.server.token_cookie_lifetime,
            token_cookie_refresh: config_file.server.token_cookie_refresh,
            ..if let Some(password) = &config_file.server.password {
                Self::from_secret(&password.hash)
            } else {
                Self::random()
            }
        }
    }

    fn from_secret(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 15;
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            token_cookie_lifetime: DEFAULT_TOKEN_LIFETIME,
            token_cookie_refresh: DEFAULT_TOKEN_REFRESH,
        }
    }

    pub fn make_token(&self) -> Result<Cookie<'static>, jsonwebtoken::errors::Error> {
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Duration::from_secs(60),
                exp: TOKEN_COOKIE_LIFETIME,
            }
            .into_timestamps(),
            &self.encoding_key,
        )?;
        let mut cookie = Cookie::new(TOKEN_COOKIE_NAME, token);
        cookie.set_path("/api");
        cookie.set_same_site(SameSite::Lax);
        cookie.set_http_only(true);
        cookie.set_max_age(Some(
            TOKEN_COOKIE_LIFETIME
                .try_into()
                .expect("TOKEN_COOKIE_LIFETIME"),
        ));
        return Ok(cookie);
    }

    pub fn validate(&self, headers: &HeaderMap) -> Result<TokenData<Claims>, (StatusCode, String)> {
        let token = extract_token(headers)?;
        let validation = jsonwebtoken::decode(&token, &self.decoding_key, &self.validation);
        validation.map_err(|error| (StatusCode::UNAUTHORIZED, format!("{error}")))
    }
}

fn extract_token(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    let Some(auth_header) = headers.get(AUTHORIZATION) else {
        let cookies = CookieJar::from_headers(headers);
        if let Some(cookie) = cookies.get(TOKEN_COOKIE_NAME) {
            return Ok(cookie.value().to_owned());
        }
        return Err((StatusCode::UNAUTHORIZED, "Missing access token".to_owned()));
    };
    let Ok(auth_header) = auth_header.to_str() else {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!("Invalid '{AUTHORIZATION}' header utf-8 string"),
        ));
    };
    let Some(token) = remove_bearer_prefix(auth_header) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!("The '{AUTHORIZATION}' header does not contain a bearer token"),
        ));
    };
    Ok(token.to_owned())
}

fn remove_bearer_prefix(auth_header: &str) -> Option<&str> {
    static PREFIX: &str = "Bearer ";
    if auth_header.len() >= PREFIX.len() && auth_header[..PREFIX.len()].eq_ignore_ascii_case(PREFIX)
    {
        Some(&auth_header[PREFIX.len()..])
    } else {
        None
    }
}

impl AuthConfig {
    fn random() -> Self {
        let secret = Uuid::new_v4();
        Self::from_secret(secret.as_bytes())
    }
}

#[derive(Clone)]
pub struct AuthLayer {
    pub auth_config: Arc<AuthConfig>,
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
            let token_data = match auth_config.validate(request.headers()) {
                Ok(token_data) => token_data,
                Err(error) => return Ok(error.into_response()),
            };

            let response = inner.call(request).await?;
            return Ok(refresh_auth_token(auth_config, token_data, response));
        })
    }
}

fn refresh_auth_token(
    auth_config: Arc<AuthConfig>,
    token_data: TokenData<Claims>,
    response: Response<Body>,
) -> Response<Body> {
    let Ok(expiration) = token_data.claims.exp.duration_since(SystemTime::now()) else {
        return response;
    };
    if expiration > TOKEN_COOKIE_REFRESH {
        debug!("The auth cookie expires in {expiration:?} > {TOKEN_COOKIE_REFRESH:?}");
        return response;
    }

    let Ok(token) = auth_config
        .make_token()
        .inspect_err(|error| warn!("Failed to create refreshed token: {error}"))
    else {
        return response;
    };

    let cookies = CookieJar::from_headers(response.headers()).add(token);
    return (cookies, response).into_response();
}

#[cfg(test)]
mod tests {

    use std::time::Duration;
    use std::time::SystemTime;

    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;
    use http::Request;
    use http::Response;
    use http::StatusCode;
    use http::header::AUTHORIZATION;
    use jsonwebtoken::Header;
    use terrazzo::axum;
    use terrazzo::http;

    use super::AuthConfig;
    use super::Claims;
    use super::jwt_timestamp::Timestamp;

    #[tokio::test]
    async fn missing_authorization_header() {
        let auth_config = AuthConfig::random();
        let request = make_request(|b| b);
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("Missing access token", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn missing_bearer_token() {
        let auth_config = AuthConfig::random();
        let request = make_request(|b| b.header(AUTHORIZATION, "blabla"));
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!(
            "The 'authorization' header does not contain a bearer token",
            get_body(response).await.unwrap()
        );
    }

    #[tokio::test]
    async fn invalid_bearer_token() {
        let auth_config = AuthConfig::random();
        let request = make_request(|b| b.header(AUTHORIZATION, "Bearer blabla"));
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("InvalidToken", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn valid_token() {
        let auth_config = AuthConfig::random();
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Duration::from_secs(60),
                exp: Duration::from_secs(3600),
            }
            .into_timestamps(),
            &auth_config.encoding_key,
        )
        .unwrap();

        let request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let _token_data: jsonwebtoken::TokenData<Claims> =
            auth_config.validate(request.headers()).unwrap();
    }

    #[tokio::test]
    async fn early_token() {
        let auth_config = AuthConfig::random();
        let now = SystemTime::now();
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Timestamp::from(now + Duration::from_secs(60)),
                exp: Timestamp::from(now + Duration::from_secs(3600)),
            },
            &auth_config.encoding_key,
        )
        .unwrap();

        let request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("ImmatureSignature", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn expired_token() {
        let auth_config = AuthConfig::random();
        let now = SystemTime::now();
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Timestamp::from(now - Duration::from_secs(3600)),
                exp: Timestamp::from(now - Duration::from_secs(60)),
            },
            &auth_config.encoding_key,
        )
        .unwrap();

        let request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("ExpiredSignature", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn bad_signature_token() {
        let auth_config = AuthConfig::random();
        let auth_config2 = AuthConfig::random();
        let now = SystemTime::now();
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Timestamp::from(now - Duration::from_secs(3600)),
                exp: Timestamp::from(now - Duration::from_secs(60)),
            },
            &auth_config2.encoding_key,
        )
        .unwrap();

        let request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = auth_config
            .validate(request.headers())
            .unwrap_err()
            .into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("InvalidSignature", get_body(response).await.unwrap());
    }

    fn make_request(
        f: impl FnOnce(http::request::Builder) -> http::request::Builder,
    ) -> Request<Body> {
        f(Request::builder()
            .method("GET")
            .uri("http://localhost/authenticated"))
        .body(Body::empty())
        .unwrap()
    }

    async fn get_body(response: Response<Body>) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = to_bytes(response.into_body(), 1024).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }
}
