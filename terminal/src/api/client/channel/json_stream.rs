use std::future::Ready;
use std::future::ready;

use futures::Stream;
use futures::StreamExt as _;
use futures::future;
use futures::stream::once;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;

use crate::api::NEWLINE;

pub fn to_json_stream<O: for<'t> Deserialize<'t>>(
    body: web_sys::ReadableStream,
) -> impl Stream<Item = Result<O, JsonStreamError>> {
    let stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
    let stream = stream.scan(JsonStreamState::default(), |state, chunk| {
        ready(process_chunks(state, chunk))
    });
    return stream.flatten();
}

fn process_chunks<O: for<'t> Deserialize<'t>>(
    state: &mut JsonStreamState,
    chunk: Result<JsValue, JsValue>,
) -> Option<impl Stream<Item = Result<O, JsonStreamError>> + use<O>> {
    let buffer = match state {
        JsonStreamState::EOS => return None,
        JsonStreamState::Buffer(buffer) => buffer,
    };

    let chunk = match chunk {
        Ok(chunk) => chunk,
        Err(error) => {
            *state = JsonStreamState::EOS;
            return Some(once(ready(Err(JsonStreamError::Error(error)))).left_stream());
        }
    };

    let Some(chunk) = chunk.dyn_ref::<Uint8Array>() else {
        return Some(once(ready(Err(JsonStreamError::BadChunk(chunk)))).left_stream());
    };

    let old_len = buffer.len();
    let new_len = old_len + chunk.length() as usize;
    buffer.resize(new_len, 0);
    chunk.copy_to(&mut buffer[old_len..new_len]);

    return Some(futures::stream::iter(process_chunk(buffer)).right_stream());
}

fn process_chunk<O: for<'t> Deserialize<'t>>(
    buffer: &mut Vec<u8>,
) -> impl Iterator<Item = Result<O, JsonStreamError>> + use<O> {
    let mut consumed = 0;
    let mut objects = vec![];
    for chunk in buffer.split_inclusive(|c| *c == NEWLINE) {
        if chunk.last() == Some(&NEWLINE) {
            consumed += chunk.len();
            objects.push(parse_chunk(&chunk[..chunk.len() - 1]));
        }
    }
    buffer.drain(..consumed);
    return objects.into_iter();
}

fn parse_chunk<O: for<'t> Deserialize<'t>>(chunk: &[u8]) -> Result<O, JsonStreamError> {
    Ok(serde_json::from_slice(chunk)?)
}

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

    #[error("[{n}] {0}", n = self.name())]
    Json(#[from] serde_json::Error),
}
