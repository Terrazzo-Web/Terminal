use std::future::ready;
use std::time::Duration;

use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use futures::stream::once;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use terrazzo::prelude::Closure;
use tracing::debug;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::RequestInit;
use web_sys::js_sys::Uint8Array;

use crate::api::NEWLINE;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::ThenRequest as _;
use crate::api::client::request::send_request;
use crate::api::client::request::set_correlation_id;
use crate::api::client::request::set_headers;

pub async fn get_download_stream<O, F, FF>(
    url: &str,
    correlation_id: String,
    on_request: F,
) -> Result<impl Stream<Item = Result<O, DownloadItemError>> + use<O, F, FF>, DownloadError>
where
    O: for<'t> Deserialize<'t>,
    F: Fn() -> FF,
    FF: FnOnce(&RequestInit),
{
    let response = download_request(url, &correlation_id, on_request).await?;
    let body = response.body().ok_or(DownloadError::MissingResponseBody)?;
    let stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
    let stream = stream.scan(DownloadStreamState::default(), |state, chunk| {
        ready(process_chunks(state, chunk))
    });
    return Ok(stream.flatten());
}

async fn download_request<F, FF>(
    url: &str,
    correlation_id: &str,
    on_request: F,
) -> Result<web_sys::Response, DownloadError>
where
    F: Fn() -> FF,
    FF: FnOnce(&RequestInit),
{
    let mut retry_delay = Duration::from_millis(50);
    let mut last_error = DownloadError::Unknown;
    while retry_delay < Duration::from_secs(5) {
        let on_request = set_headers(set_correlation_id(correlation_id)).then(on_request());
        let error = match send_request(Method::GET, url, on_request).await {
            Ok(response) => return Ok(response),
            Err(error) => error,
        };
        match &error {
            SendRequestError::Message { message } if message == "Unknown channel ID" => (),
            _ => return Err(error.into()),
        }
        debug!("Failed to open channel, retrying...: {error}");
        last_error = error.into();

        let (tx, rx) = oneshot::channel();
        let handler = Closure::once(|| {
            let _ = tx.send(());
        });
        let _handle: i32 = web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                handler.as_ref().unchecked_ref(),
                retry_delay.as_millis() as i32,
            )
            .map_err(DownloadError::RetryTimer)?;
        let _ = rx.await;
        drop(handler);
        retry_delay = 2 * retry_delay;
    }
    return Err(last_error);
}

fn process_chunks<O: for<'t> Deserialize<'t>>(
    state: &mut DownloadStreamState,
    chunk: Result<JsValue, JsValue>,
) -> Option<impl Stream<Item = Result<O, DownloadItemError>> + use<O>> {
    let buffer = match state {
        DownloadStreamState::EOS => return None,
        DownloadStreamState::Buffer(buffer) => buffer,
    };

    let chunk = match chunk {
        Ok(chunk) => chunk,
        Err(error) => {
            *state = DownloadStreamState::EOS;
            return Some(once(ready(Err(DownloadItemError::Error(error)))).left_stream());
        }
    };

    let Some(chunk) = chunk.dyn_ref::<Uint8Array>() else {
        return Some(once(ready(Err(DownloadItemError::BadChunk(chunk)))).left_stream());
    };

    let old_len = buffer.len();
    let new_len = old_len + chunk.length() as usize;
    buffer.resize(new_len, 0);
    chunk.copy_to(&mut buffer[old_len..new_len]);

    return Some(futures::stream::iter(process_chunk(buffer)).right_stream());
}

fn process_chunk<O: for<'t> Deserialize<'t>>(
    buffer: &mut Vec<u8>,
) -> impl Iterator<Item = Result<O, DownloadItemError>> + use<O> {
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

fn parse_chunk<O: for<'t> Deserialize<'t>>(chunk: &[u8]) -> Result<O, DownloadItemError> {
    Ok(serde_json::from_slice(chunk)?)
}

enum DownloadStreamState {
    EOS,
    Buffer(Vec<u8>),
}

impl Default for DownloadStreamState {
    fn default() -> Self {
        Self::Buffer(vec![])
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadItemError {
    #[error("[{n}] JSON Stream failed with: {0:?}", n = self.name())]
    Error(JsValue),

    #[error("[{n}] Chunk is not a string: {0:?}", n = self.name())]
    BadChunk(JsValue),

    #[error("[{n}] {0}", n = self.name())]
    Json(#[from] serde_json::Error),
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("[{n}] Unknown error", n = self.name())]
    Unknown,

    #[error("[{n}] Failed to open JSON stream: {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] There is no response body", n = self.name())]
    MissingResponseBody,

    #[error("[{n}] Failed to set the timer for retry: {0:?}", n = self.name())]
    RetryTimer(JsValue),
}
