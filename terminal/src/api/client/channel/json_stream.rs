use std::future::ready;

use futures::Stream;
use futures::StreamExt;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;

pub fn to_json_stream<O: 'static>(
    body: web_sys::ReadableStream,
) -> impl Stream<Item = Result<O, JsonStreamError>> + 'static {
    let stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
    let stream = stream.scan(JsonStreamState::default(), |state, chunk| {
        // let buffer = match state {
        //     JsonStreamState::EOS => return None,
        //     JsonStreamState::Buffer(buffer) => buffer,
        // };

        // let chunk = match chunk {
        //     Ok(chunk) => chunk,
        //     Err(error) => {
        //         *state = JsonStreamState::EOS;
        //         return Some(Err(JsonStreamError::Error(error)));
        //     }
        // };

        // let Some(chunk) = chunk.dyn_ref::<Uint8Array>() else {
        //     return Some(Err(JsonStreamError::BadChunk(chunk)));
        // };

        // let count = chunk.length() as usize;
        // let old_len = buffer.len();
        // let new_len = old_len + count;
        // buffer.extend(std::iter::repeat(b'\0').take(count));
        // let slice = &mut buffer[old_len..new_len];
        // chunk.copy_to(slice);
        return ready(Some(Err(JsonStreamError::Error(JsValue::null()))));
    });
    check_type::<O>(&stream);
    return stream;
}

fn check_type<O>(_: &impl Stream<Item = Result<O, JsonStreamError>>) {}

enum JsonStreamState {
    EOS,
    Buffer(Vec<u8>),
}

impl Default for JsonStreamState {
    fn default() -> Self {
        Self::Buffer(vec![])
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum JsonStreamError {
    #[error("[{n}] JSON Stream failed with: {0:?}", n = self.name())]
    Error(JsValue),

    #[error("[{n}] Chunk is not a string: {0:?}", n = self.name())]
    BadChunk(JsValue),
}
