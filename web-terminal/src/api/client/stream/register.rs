use named::named;
use named::NamedEnumValues as _;
use terrazzo::prelude::OrElseLog as _;
use wasm_bindgen::JsValue;
use web_sys::Headers;
use web_sys::Response;

use super::pipe::PipeError;
use crate::api::client::send_request;
use crate::api::client::Method;
use crate::api::client::SendRequestError;
use crate::api::client::BASE_URL;
use crate::api::RegisterTerminalRequest;

/// Instructs the server to include `terminal_id`'s data in the pipe.
#[named]
pub async fn register(request: RegisterTerminalRequest) -> Result<(), RegisterError> {
    let json = serde_json::to_string(&request)?;
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/stream/{REGISTER}"),
        move |request| {
            let headers = Headers::new().or_throw("Headers::new()");
            headers
                .set("content-type", "application/json")
                .or_throw("Set 'content-type'");
            request.set_headers(headers.as_ref());
            request.set_body(&JsValue::from_str(&json));
        },
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum RegisterError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    PipeError(#[from] PipeError),

    #[error("[{n}] {0}", n = self.name())]
    InvalidJson(#[from] serde_json::Error),
}
