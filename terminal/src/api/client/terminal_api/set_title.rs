use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;
use crate::api::client::terminal_api::BASE_TERMINAL_URL;
use crate::api::shared::terminal_schema::SetTitleRequest;
use crate::api::shared::terminal_schema::TabTitle;
use crate::api::shared::terminal_schema::TerminalAddress;

#[nameth]
pub async fn set_title(
    terminal: &TerminalAddress,
    title: TabTitle<String>,
) -> Result<(), SetTitleError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_TERMINAL_URL}/{SET_TITLE}"),
        set_json_body(&SetTitleRequest { terminal, title })?,
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
