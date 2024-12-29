use nameth::nameth;
use nameth::NamedEnumValues as _;
use wasm_bindgen::JsValue;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::terminal_id::TerminalId;

#[nameth]
pub async fn write(terminal_id: &TerminalId, data: String) -> Result<(), WriteError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{WRITE}/{terminal_id}"),
        |request| request.set_body(&JsValue::from_str(&data)),
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}
