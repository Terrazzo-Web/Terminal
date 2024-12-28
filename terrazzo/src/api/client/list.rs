use named::named;
use named::NamedEnumValues as _;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::api::TerminalDef;

#[named]
pub async fn list() -> Result<Vec<TerminalDef>, ListError> {
    let response: Response =
        send_request(Method::GET, format!("{BASE_URL}/{LIST}"), |_| {}).await?;
    let Some(body) = response.body() else {
        return Err(ListError::MissingResponseBody);
    };
    let mut reader = wasm_streams::ReadableStream::from_raw(body);
    let mut reader = reader.get_reader();

    let mut data = vec![];
    loop {
        let next = reader.read().await;
        let Some(next) = next.map_err(ListError::ReadError)? else {
            break;
        };
        let Some(next) = next.dyn_ref::<Uint8Array>() else {
            return Err(ListError::InvalidChunk(next));
        };

        let count = next.length() as usize;
        let old_length = data.len();
        let new_length = old_length + count;
        data.extend(std::iter::repeat(b'\0').take(count));
        next.copy_to(&mut data[old_length..new_length]);
    }

    let terminal_ids: Vec<TerminalDef> =
        serde_json::from_slice(&data).map_err(ListError::InvalidJson)?;
    Ok(terminal_ids)
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum ListError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{}] Missing response body", self.name())]
    MissingResponseBody,

    #[error("[{}] Stream failed: {0:?}", self.name())]
    ReadError(JsValue),

    #[error("[{}] Chunk is not a byte array: {0:?}", self.name())]
    InvalidChunk(JsValue),

    #[error("[{}] Invalid JSON result: {0:?}", self.name())]
    InvalidJson(serde_json::Error),
}
