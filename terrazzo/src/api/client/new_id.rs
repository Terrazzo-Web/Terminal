use named::named;
use named::NamedEnumValues as _;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;

#[named]
pub async fn new_id() -> Result<String, NewIdError> {
    let response: Response =
        send_request(Method::POST, format!("{BASE_URL}/{NEW_ID}"), |_| {}).await?;
    let result = response
        .text()
        .map_err(|_| NewIdError::MissingResponseBody)?;
    let result = JsFuture::from(result)
        .await
        .map_err(|_| NewIdError::FailedResponseBody)?;
    let result = result.as_string().ok_or(NewIdError::InvalidUtf8Id)?;
    Ok(result)
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

    #[error("[{n}] The returned ID is not a valid UTF-8 string", n = self.name())]
    InvalidUtf8Id,
}
