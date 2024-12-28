use named::named;
use named::NamedEnumValues as _;
use terrazzo::prelude::OrElseLog as _;
use wasm_bindgen::JsValue;
use web_sys::Headers;
use web_sys::Response;

use super::send_request;
use super::set_content_type_json;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::api::TabTitle;
use crate::terminal_id::TerminalId;

#[named]
pub async fn set_title(
    terminal_id: &TerminalId,
    title: TabTitle<String>,
) -> Result<(), SetTitleError> {
    let body = serde_json::to_string(&title)?;
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/{SET_TITLE}/{terminal_id}"),
        move |request| {
            let mut headers = Headers::new().or_throw("Headers::new()");
            set_content_type_json(&mut headers);
            request.set_headers(headers.as_ref());
            request.set_body(&JsValue::from_str(&body));
        },
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}
