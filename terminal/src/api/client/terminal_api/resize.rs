use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;
use crate::api::client::terminal_api::BASE_TERMINAL_URL;
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
        format!("{BASE_TERMINAL_URL}/{RESIZE} "),
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
