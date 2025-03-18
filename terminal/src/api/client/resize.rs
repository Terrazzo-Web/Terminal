use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use super::set_json_body;
use crate::api::Size;
use crate::terminal_id::TerminalId;

#[nameth]
pub async fn resize(
    terminal_id: &TerminalId,
    size: Size,
    first_resize: bool,
) -> Result<(), ResizeError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{RESIZE}/{terminal_id}"),
        set_json_body(&(size, first_resize))?,
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
