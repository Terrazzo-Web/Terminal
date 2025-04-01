use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::request::BASE_URL;
use super::request::Method;
use super::request::SendRequestError;
use super::request::send_request;
use super::request::set_json_body;
use crate::api::TerminalAddress;
use crate::api::WriteRequest;

#[nameth]
pub async fn write(terminal: &TerminalAddress, data: String) -> Result<(), WriteError> {
    let _: Response = send_request(
        Method::POST,
        &format!("{BASE_URL}/{WRITE}"),
        set_json_body(&WriteRequest { terminal, data })?,
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}
