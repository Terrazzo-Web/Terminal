use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::Response;
use web_sys::js_sys::Uint8Array;

use super::BASE_URL;
use super::Method;
use super::SendRequestError;
use super::send_request;
use crate::api::TerminalDef;

#[nameth]
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ListError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] Missing response body", n = self.name())]
    MissingResponseBody,

    #[error("[{n}] Stream failed: {0:?}", n = self.name())]
    ReadError(JsValue),

    #[error("[{n}] Chunk is not a byte array: {0:?}", n = self.name())]
    InvalidChunk(JsValue),

    #[error("[{n}] Invalid JSON result: {0:?}", n = self.name())]
    InvalidJson(serde_json::Error),
}
