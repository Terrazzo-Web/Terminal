use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use axum::body::Body;
use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
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
use uuid::Uuid;

use super::TOKEN_COOKIE_NAME;

pub struct AuthConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
struct Claims<T = Timestamp> {
    exp: T,
    nbf: T,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let secret = Uuid::new_v4();
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 15;
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        }
    }
}

impl AuthConfig {
    pub fn make_token(&self) -> Result<std::string::String, jsonwebtoken::errors::Error> {
        jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                nbf: Duration::from_secs(60),
                exp: Duration::from_secs(3600),
            }
            .into_timestamps(),
            &self.encoding_key,
        )
    }

    pub fn validate(
        self: Arc<Self>,
    ) -> impl for<'a> FnMut(&'a mut Request<Body>) -> Result<(), Response<Body>> + Clone {
        move |request| validate_impl(&self, request).map_err(|error| error.into_response())
    }
}

fn validate_impl(
    auth_config: &AuthConfig,
    request: &mut Request<Body>,
) -> Result<(), (StatusCode, String)> {
    let token = extract_token(request)?;
    let validation =
        jsonwebtoken::decode(&token, &auth_config.decoding_key, &auth_config.validation);
    validation
        .map(|_: TokenData<Claims>| ())
        .map_err(|error| (StatusCode::UNAUTHORIZED, format!("{error}")))
}

fn extract_token<'t>(request: &'t mut Request<Body>) -> Result<String, (StatusCode, String)> {
    let Some(auth_header) = request.headers().get(AUTHORIZATION) else {
        let cookies = CookieJar::from_headers(request.headers());
        if let Some(cookie) = cookies.get(TOKEN_COOKIE_NAME) {
            return Ok(cookie.value().to_owned());
        }
        return Err((StatusCode::UNAUTHORIZED, format!("Missing access token")));
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

/// Timestamp for JWT tokens, serialized as seconds since epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(SystemTime);

impl Deref for Timestamp {
    type Target = SystemTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<SystemTime> for Timestamp {
    fn from(value: SystemTime) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for SystemTime {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl Claims<Duration> {
    pub fn into_timestamps(self) -> Claims<Timestamp> {
        let now = SystemTime::now();
        Claims {
            exp: (now + self.exp).into(),
            nbf: (now - self.nbf).into(),
        }
    }
}

impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let duration: u64 = self
            .0
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?
            .as_secs();
        return duration.serialize(serializer);
    }
}

impl<'t> serde::Deserialize<'t> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'t>,
    {
        let duration = Duration::from_secs(u64::deserialize(deserializer)?);
        Ok((std::time::UNIX_EPOCH + duration).into())
    }
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
    use super::Timestamp;
    use super::validate_impl;

    #[tokio::test]
    async fn missing_authorization_header() {
        let auth_config = AuthConfig::default();
        let mut request = make_request(|b| b);
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!(
            "Missing 'authorization' header",
            get_body(response).await.unwrap()
        );
    }

    #[tokio::test]
    async fn missing_bearer_token() {
        let auth_config = AuthConfig::default();
        let mut request = make_request(|b| b.header(AUTHORIZATION, "blabla"));
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!(
            "The 'authorization' header does not contain a bearer token",
            get_body(response).await.unwrap()
        );
    }

    #[tokio::test]
    async fn invalid_bearer_token() {
        let auth_config = AuthConfig::default();
        let mut request = make_request(|b| b.header(AUTHORIZATION, "Bearer blabla"));
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("InvalidToken", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn valid_token() {
        let auth_config = AuthConfig::default();
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

        let mut request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::OK, response.status());
        assert_eq!("", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn early_token() {
        let auth_config = AuthConfig::default();
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

        let mut request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("ImmatureSignature", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn expired_token() {
        let auth_config = AuthConfig::default();
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

        let mut request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = validate_impl(&auth_config, &mut request).into_response();
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        assert_eq!("ExpiredSignature", get_body(response).await.unwrap());
    }

    #[tokio::test]
    async fn bad_signature_token() {
        let auth_config = AuthConfig::default();
        let auth_config2 = AuthConfig::default();
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

        let mut request = make_request(|b| b.header(AUTHORIZATION, &format!("Bearer {token}")));
        let response = validate_impl(&auth_config, &mut request).into_response();
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
