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
pub async fn write(terminal_id: &TerminalId, data: String) -> Result<(), WriteError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{WRITE}/{terminal_id}"),
        |request| request.set_body(&JsValue::from_str(&data)),
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),
}
