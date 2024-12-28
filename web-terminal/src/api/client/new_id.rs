use named::named;
use named::NamedEnumValues as _;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::api::TerminalDef;

#[named]
pub async fn new_id() -> Result<TerminalDef, NewIdError> {
    let response: Response =
        send_request(Method::POST, format!("{BASE_URL}/{NEW_ID}"), |_| {}).await?;
    let result = response
        .text()
        .map_err(|_| NewIdError::MissingResponseBody)?;
    let result = JsFuture::from(result)
        .await
        .map_err(|_| NewIdError::FailedResponseBody)?;
    let body = result.as_string().ok_or(NewIdError::InvalidUtf8)?;
    Ok(serde_json::from_str(&body)?)
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum NewIdError {
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
