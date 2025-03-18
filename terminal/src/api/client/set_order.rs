use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use super::set_json_body;
use crate::terminal_id::TerminalId;

#[nameth]
pub async fn set_order(tabs: Vec<TerminalId>) -> Result<(), SetOrderError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{SET_ORDER}"),
        set_json_body(&tabs)?,
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetOrderError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}
