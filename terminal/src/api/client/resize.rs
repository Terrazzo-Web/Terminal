use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::prelude::OrElseLog as _;
use wasm_bindgen::JsValue;
use web_sys::Headers;
use web_sys::Response;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use crate::api::Size;
use crate::api::client::set_content_type_json;
use crate::terminal_id::TerminalId;

#[nameth]
pub async fn resize(
    terminal_id: &TerminalId,
    size: Size,
    first_resize: bool,
) -> Result<(), ResizeError> {
    let json = serde_json::to_string(&(size, first_resize))?;
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{RESIZE}/{terminal_id}"),
        move |request| {
            let mut headers = Headers::new().or_throw("Headers::new()");
            set_content_type_json(&mut headers);
            request.set_headers(headers.as_ref());
            request.set_body(&JsValue::from_str(&json));
        },
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
