use std::time::Duration;

use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::cookie::SameSite;
use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use terrazzo::http;
use terrazzo::http::HeaderMap;
use uuid::Uuid;

use self::http::StatusCode;
use self::http::header::AUTHORIZATION;
use self::jwt_timestamp::Timestamp;
use super::config::server::ServerConfig;

mod jwt_timestamp;
pub mod layer;
mod tests;

static TOKEN_COOKIE_NAME: &str = "slt";

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
    token_lifetime: Duration,
    token_refresh: Duration,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct Claims<T = Timestamp> {
    exp: T,
    nbf: T,
}

impl AuthConfig {
    pub fn new(server_config: &ServerConfig) -> Self {
        Self {
            token_lifetime: server_config.token_lifetime,
            token_refresh: server_config.token_refresh,
            ..if let Some(password) = &server_config.password {
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
            token_lifetime: DEFAULT_TOKEN_LIFETIME,
            token_refresh: DEFAULT_TOKEN_REFRESH,
        }
    }

    pub fn make_token(&self) -> Result<Cookie<'static>, jsonwebtoken::errors::Error> {
        let token = jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Duration::from_secs(60),
                exp: self.token_lifetime,
            }
            .into_timestamps(),
            &self.encoding_key,
        )?;
        let mut cookie = Cookie::new(TOKEN_COOKIE_NAME, token);
        cookie.set_path("/api");
        cookie.set_same_site(SameSite::Lax);
        cookie.set_http_only(true);
        cookie.set_max_age(Some(
            self.token_lifetime.try_into().expect("token_lifetime"),
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
