use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use crate::api::client_address::ClientAddress;

#[nameth]
pub async fn remotes() -> Result<Vec<ClientAddress>, RemotesError> {
    let response: Response =
        send_request(Method::GET, format!("{BASE_URL}/{REMOTES}"), |_| {}).await?;
    let result = response
        .text()
        .map_err(|_| RemotesError::MissingResponseBody)?;
    let result = JsFuture::from(result)
        .await
        .map_err(|_| RemotesError::FailedResponseBody)?;
    let body = result.as_string().ok_or(RemotesError::InvalidUtf8)?;
    Ok(serde_json::from_str(&body)?)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RemotesError {
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
