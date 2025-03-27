use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::axum::extract::FromRequestParts;
use terrazzo::http::StatusCode;
use terrazzo::http::header::ToStrError;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;

use crate::api::CORRELATION_ID;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(Arc<str>);

/// [CorrelationId] can be provided as a header.
impl<S: Sync> FromRequestParts<S> for CorrelationId {
    type Rejection = HttpError<CorrelationIdError>;

    async fn from_request_parts(
        parts: &mut terrazzo::http::request::Parts,
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum CorrelationIdError {
    #[error("[{n}] Missing header '{CORRELATION_ID}'", n = self.name())]
    MissingCorrelationId,

    #[error("[{n}] Invalid string: {0}", n = self.name())]
    InvalidString(ToStrError),
}

impl IsHttpError for CorrelationIdError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
