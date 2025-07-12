#![cfg(test)]

use std::time::Duration;
use std::time::SystemTime;

use jsonwebtoken::Header;
use terrazzo::axum;
use terrazzo::axum::body::Body;
use terrazzo::axum::body::to_bytes;
use terrazzo::axum::response::IntoResponse;
use terrazzo::http::Request;
use terrazzo::http::Response;
use terrazzo::http::StatusCode;
use terrazzo::http::header::AUTHORIZATION;

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

    let request = make_request(|b| b.header(AUTHORIZATION, format!("Bearer {token}")));
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

    let request = make_request(|b| b.header(AUTHORIZATION, format!("Bearer {token}")));
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

    let request = make_request(|b| b.header(AUTHORIZATION, format!("Bearer {token}")));
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

    let request = make_request(|b| b.header(AUTHORIZATION, format!("Bearer {token}")));
    let response = auth_config
        .validate(request.headers())
        .unwrap_err()
        .into_response();
    assert_eq!(StatusCode::UNAUTHORIZED, response.status());
    assert_eq!("InvalidSignature", get_body(response).await.unwrap());
}

fn make_request(f: impl FnOnce(http::request::Builder) -> http::request::Builder) -> Request<Body> {
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
