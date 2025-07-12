#![cfg(feature = "terminal")]

use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::super::request::BASE_URL;
use super::super::request::Method;
use super::super::request::SendRequestError;
use super::super::request::send_request;
use super::super::request::set_json_body;
use crate::api::shared::terminal_schema::ResizeRequest;
use crate::api::shared::terminal_schema::Size;
use crate::api::shared::terminal_schema::TerminalAddress;

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
