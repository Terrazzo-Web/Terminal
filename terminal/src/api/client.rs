use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Serialize;
use terrazzo::prelude::OrElseLog;
use terrazzo::prelude::XSignal;
use terrazzo::prelude::XString;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Headers;
use web_sys::Request;
use web_sys::RequestInit;
use web_sys::RequestMode;
use web_sys::Response;

use super::APPLICATION_JSON;
use super::TabTitle;
use super::TerminalDefImpl;

pub mod new_id;
pub mod remotes;
pub mod resize;
pub mod set_order;
pub mod set_title;
pub mod stream;
pub mod terminals;
pub mod write;

const BASE_URL: &str = "/api";

async fn send_request(
    method: Method,
    url: String,
    on_request: impl FnOnce(&RequestInit),
) -> Result<Response, SendRequestError> {
    let request = RequestInit::new();
    request.set_method(method.name());
    request.set_mode(RequestMode::SameOrigin);
    on_request(&request);
    let request = Request::new_with_str_and_init(&url, &request);
    let request = request.map_err(|error| SendRequestError::InvalidUrl { url, error })?;
    let window = web_sys::window().or_throw("window");
    let promise = window.fetch_with_request(&request);
    let response = JsFuture::from(promise)
        .await
        .map_err(|error| SendRequestError::RequestError { error })?;
    let response: Response = response
        .dyn_into()
        .map_err(|error| SendRequestError::UnexpectedResponseObject { error })?;
    if !response.ok() {
        warn!("Request failed: {}", response.status());
        let message = response
            .text()
            .map_err(|_| SendRequestError::MissingErrorBody)?;
        let message = JsFuture::from(message)
            .await
            .map_err(|_| SendRequestError::FailedErrorBody)?;
        let message = message
            .as_string()
            .ok_or(SendRequestError::InvalidErrorBody)?;
        return Err(SendRequestError::Message { message });
    }
    return Ok(response);
}

#[nameth]
#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
enum Method {
    GET,
    POST,
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SendRequestError {
    #[error("[{}] Invalid url='{url}': {error:?}", self.name())]
    InvalidUrl { url: String, error: JsValue },

    #[error("[{}] {error:?}", self.name())]
    RequestError { error: JsValue },

    #[error("[{}] Unexpected {error:?}", self.name())]
    UnexpectedResponseObject { error: JsValue },

    #[error("[{}] Missing error message", self.name() )]
    MissingErrorBody,

    #[error("[{}] Failed to download error message", self.name() )]
    FailedErrorBody,

    #[error("[{}] Failed to parse error message", self.name() )]
    InvalidErrorBody,

    #[error("[{}] {message}", self.name())]
    Message { message: String },
}

pub type LiveTerminalDef = TerminalDefImpl<XSignal<TabTitle<XString>>>;

fn set_json_body<T>(body: &T) -> serde_json::Result<impl Fn(&RequestInit)>
where
    T: ?Sized + Serialize,
{
    let body = serde_json::to_string(body)?;
    Ok(move |request: &RequestInit| {
        set_headers(request, set_content_type_json);
        request.set_body(&JsValue::from_str(&body));
    })
}

fn set_headers(request: &RequestInit, f: impl FnOnce(&mut Headers)) {
    let mut headers = Headers::new().or_throw("Headers::new()");
    f(&mut headers);
    request.set_headers(headers.as_ref());
}

fn set_content_type_json(headers: &mut Headers) {
    headers
        .set("content-type", APPLICATION_JSON)
        .or_throw("Set 'content-type'");
}
