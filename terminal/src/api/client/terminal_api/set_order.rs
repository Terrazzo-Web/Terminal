use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use crate::api::client::request::BASE_URL;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;
use crate::api::shared::terminal_schema::TerminalAddress;

#[nameth]
pub async fn set_order(tabs: Vec<TerminalAddress>) -> Result<(), SetOrderError> {
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
