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
pub async fn set_order(tabs: Vec<TerminalId>) -> Result<(), SetOrderError> {
    let body = serde_json::to_string(&tabs)?;
    let _: Response = send_request(Method::POST, format!("{BASE_URL}/{SET_ORDER}"), |request| {
        request.set_body(&JsValue::from_str(&body))
    })
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum SetOrderError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}
