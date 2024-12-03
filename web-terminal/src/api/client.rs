use named::named;
use named::NamedEnumValues as _;
use terrazzo::prelude::OrElseLog as _;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Request;
use web_sys::RequestInit;
use web_sys::RequestMode;
use web_sys::Response;

use crate::api::ERROR_HEADER;

pub mod list;
pub mod new_id;
pub mod resize;
pub mod set_title;
pub mod stream;
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
        return Err(match response.headers().get(ERROR_HEADER) {
            Ok(Some(header)) => SendRequestError::Header { header },
            Ok(None) => SendRequestError::MissingErrorHeader,
            Err(error) => SendRequestError::InvalidHeader {
                details: error
                    .as_string()
                    .unwrap_or_else(|| "Unknown error".to_string()),
            },
        });
    }
    return Ok(response);
}

#[named]
#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
enum Method {
    GET,
    POST,
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum SendRequestError {
    #[error("[{}] Invalid url='{url}': {error:?}", self.name())]
    InvalidUrl { url: String, error: JsValue },

    #[error("[{}] {error:?}", self.name())]
    RequestError { error: JsValue },

    #[error("[{}] Unexpected {error:?}", self.name())]
    UnexpectedResponseObject { error: JsValue },

    #[error("[{}] {header}", self.name())]
    Header { header: String },

    #[error("[{}] Missing error header", self.name() )]
    MissingErrorHeader,

    #[error("[{}] {details}", self.name())]
    InvalidHeader { details: String },
}
