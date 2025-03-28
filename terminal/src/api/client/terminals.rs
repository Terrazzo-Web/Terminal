use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;
use web_sys::js_sys::Uint8Array;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use crate::api::TerminalDef;

#[nameth]
pub async fn terminals() -> Result<Vec<TerminalDef>, ListTerminalsError> {
    let response = send_request(Method::GET, format!("{BASE_URL}/{TERMINALS}"), |_| {}).await?;
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
