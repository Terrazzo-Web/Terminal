use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use super::set_json_body;
use crate::api::TabTitle;
use crate::terminal_id::TerminalId;

#[nameth]
pub async fn set_title(
    terminal_id: &TerminalId,
    title: TabTitle<String>,
) -> Result<(), SetTitleError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{SET_TITLE}/{terminal_id}"),
        set_json_body(&title)?,
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}
