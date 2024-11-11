use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::response::IntoResponse;
use axum::response::Response;
use http::header::ToStrError;
use http::StatusCode;
use named::named;
use named::NamedEnumValues as _;

use super::into_error;
use crate::api::CORRELATION_ID;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(Arc<str>);

#[async_trait]
impl<S> FromRequestParts<S> for CorrelationId {
    type Rejection = CorrelationIdError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let correlation_id = parts
            .headers
            .get(CORRELATION_ID)
            .ok_or(CorrelationIdError::MissingCorrelationId)?;
        let correlation_id = correlation_id
            .to_str()
            .map_err(CorrelationIdError::InvalidString)?;
        return Ok(CorrelationId(correlation_id.into()));
    }
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum CorrelationIdError {
    #[error("[{n}] Missing header '{CORRELATION_ID}'", n = self.name() )]
    MissingCorrelationId,

    #[error("[{n}] Invalid string: {0}", n = self.name())]
    InvalidString(ToStrError),
}

impl IntoResponse for CorrelationIdError {
    fn into_response(self) -> Response {
        into_error(StatusCode::BAD_REQUEST)(self)
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
