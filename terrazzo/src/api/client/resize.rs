use named::named;
use named::NamedEnumValues as _;
use wasm_bindgen::JsValue;
use web_sys::Headers;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::api::Size;
use crate::terminal_id::TerminalId;

#[named]
pub async fn resize(terminal_id: &TerminalId, size: Size) -> Result<(), ResizeError> {
    let json = serde_json::to_string(&size)?;
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{RESIZE}/{terminal_id}"),
        move |request| {
            let headers = Headers::new().expect("Headers::new()");
            headers
                .set("content-type", "application/json")
                .expect("Set 'content-type'");
            request.set_headers(headers.as_ref());
            request.set_body(&JsValue::from_str(&json));
        },
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum ResizeError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{}] {0}", self.name())]
    InvalidJson(#[from] serde_json::Error),
}
