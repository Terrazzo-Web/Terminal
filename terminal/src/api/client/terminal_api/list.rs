#![cfg(feature = "terminal")]

use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen_futures::JsFuture;

use super::super::request::BASE_URL;
use super::super::request::Method;
use super::super::request::SendRequestError;
use super::super::request::send_request;
use crate::api::shared::terminal_schema::TerminalDef;

#[nameth]
pub async fn list() -> Result<Vec<TerminalDef>, ListTerminalsError> {
    let response = send_request(Method::GET, format!("{BASE_URL}/{LIST}"), |_| {}).await?;
    let response = response
        .text()
        .map_err(|_| ListTerminalsError::MissingResponseBody)?;
    let response = JsFuture::from(response)
        .await
        .map_err(|_| ListTerminalsError::FailedResponseBody)?;
    let response = response
        .as_string()
        .ok_or(ListTerminalsError::InvalidUtf8)?;
    Ok(serde_json::from_str(&response)?)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ListTerminalsError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] Missing response body", n = self.name())]
    MissingResponseBody,

    #[error("[{n}] Failed to download the response body", n = self.name())]
    FailedResponseBody,

    #[error("[{n}] The response body is not a valid UTF-8 string", n = self.name())]
    InvalidUtf8,

    #[error("[{n}] {0}", n = self.name())]
    InvalidJson(#[from] serde_json::Error),
}
