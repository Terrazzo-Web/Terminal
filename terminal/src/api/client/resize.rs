use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::request::BASE_URL;
use super::request::Method;
use super::request::SendRequestError;
use super::request::send_request;
use super::request::set_json_body;
use crate::api::ResizeRequest;
use crate::api::Size;
use crate::api::TerminalAddress;

#[nameth]
pub async fn resize(
    terminal: &TerminalAddress,
    size: Size,
    force: bool,
) -> Result<(), ResizeError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{RESIZE} "),
        set_json_body(&ResizeRequest {
            terminal,
            size,
            force,
        })?,
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ResizeError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    InvalidJson(#[from] serde_json::Error),
}
