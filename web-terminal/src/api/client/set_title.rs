use named::named;
use named::NamedEnumValues as _;
use wasm_bindgen::JsValue;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::terminal_id::TerminalId;

#[named]
pub async fn set_title(terminal_id: &TerminalId, title: String) -> Result<(), SetTitleError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{SET_TITLE}/{terminal_id}"),
        |request| request.set_body(&JsValue::from_str(&title)),
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}
