use std::sync::Arc;

use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use terrazzo::axum::body::Body;
use terrazzo::axum::response::IntoResponse;
use terrazzo::http::Request;
use terrazzo::http::Response;
use terrazzo::http::StatusCode;
use terrazzo::http::header::AUTHORIZATION;
use trz_gateway_server::server::Server;

// pub struct AuthConfig {}

pub fn validate(
    server: Arc<Server>,
) -> impl for<'a> FnMut(&'a mut Request<Body>) -> Result<(), Response<Body>> + Clone {
    move |request| validate_impl(&server, request).map_err(StatusCode::into_response)
}

pub fn validate_impl(_server: &Server, request: &mut Request<Body>) -> Result<(), StatusCode> {
    let Some(auth_header) = request.headers().get(AUTHORIZATION) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Ok(auth_header) = auth_header.to_str() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(token) = remove_bearer_prefix(auth_header) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let _validation: jsonwebtoken::errors::Result<TokenData<()>> = jsonwebtoken::decode(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    Ok(())
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
