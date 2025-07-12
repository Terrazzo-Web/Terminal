use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::oneshot;
use futures::select;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::prelude::OrElseLog as _;
use terrazzo::prelude::diagnostics;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Response;
use web_sys::js_sys::Uint8Array;

use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use self::diagnostics::info;
use self::diagnostics::info_span;
use self::diagnostics::warn;
use super::DISPATCHERS;
use super::ShutdownPipe;
use super::dispatch::dispatch;
use super::keepalive::keepalive;
use crate::api::KEEPALIVE_TTL_HEADER;
use crate::api::client::request::BASE_URL;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_correlation_id;
use crate::api::client::request::set_headers;

/// Spawns the pipe in the background.
#[nameth]
pub async fn pipe(correlation_id: Arc<str>) -> Result<oneshot::Sender<()>, PipeError> {
    let span = info_span!("Pipe", %correlation_id);
    async move {
        info!("Start");
        let response = send_request(
            Method::POST,
            format!("{BASE_URL}/stream/{PIPE}"),
            set_headers(set_correlation_id(correlation_id.as_ref())),
        )
        .await?;
        let Some(stream) = response.body() else {
            return Err(PipeError::EmptyStream);
        };
        let keepalive_ttl = get_keepalive_ttl(&response)?;

        let (end_of_pipe_tx, end_of_pipe_rx) = oneshot::channel();
        keepalive(keepalive_ttl, correlation_id.clone(), end_of_pipe_rx);

        info!("Streaming");
        let (tx, rx) = oneshot::channel();
        let streaming_task = async move {
            // Close all the stream dispatchers if the pipe fails.
            defer! { close_dispatchers(&correlation_id); };
            defer! { let _ = end_of_pipe_tx.send(()); };
            if let Err(error) = pipe_impl(rx, stream).await {
                warn!("Pipe failed: {error}");
            }
            info!("Closed");
        };
        spawn_local(streaming_task.in_current_span());
        return Ok(tx);
    }
    .instrument(span)
    .await
}

fn get_keepalive_ttl(response: &Response) -> Result<Duration, PipeError> {
    match response.headers().get(KEEPALIVE_TTL_HEADER) {
        Ok(Some(ttl)) => ttl
            .parse()
            .map(Duration::from_secs)
            .map_err(|_| PipeError::KeepaliveTtl),
        _ => Err(PipeError::KeepaliveTtl),
    }
}

async fn pipe_impl(
    mut shutdown: oneshot::Receiver<()>,
    stream: web_sys::ReadableStream,
) -> Result<(), PipeError> {
    let mut stream = wasm_streams::ReadableStream::from_raw(stream);
    let mut stream = stream.get_reader().into_stream().ready_chunks(10);

    let mut buffer = vec![];
    loop {
        let next = select! {
            next = stream.next() => next,
            _ = shutdown => return Ok(()),
        };
        let Some(next) = next else {
            return Ok(());
        };
        for chunk in next {
            let chunk = chunk.map_err(PipeError::ReadError)?;
            let Some(chunk) = chunk.dyn_ref::<Uint8Array>() else {
                return Err(PipeError::InvalidChunk(chunk));
            };
            let count = chunk.length() as usize;
            let old_len = buffer.len();
            let new_len = old_len + count;
            buffer.extend(std::iter::repeat_n(b'\0', count));
            let slice = &mut buffer[old_len..new_len];
            chunk.copy_to(slice);
        }
        dispatch(&mut buffer).await;
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PipeError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] Pipe is an empty stream", n = self.name())]
    EmptyStream,

    #[error("[{n}] Chunk is not a byte array: {0:?}", n = self.name())]
    InvalidChunk(JsValue),

    #[error("[{n}] Stream failed: {0:?}", n = self.name())]
    ReadError(JsValue),

    #[error("[{n}] Pipe canceled", n = self.name())]
    Canceled,

    #[error("[{n}] Missing header {KEEPALIVE_TTL_HEADER}", n = self.name())]
    KeepaliveTtl,
}

fn close_dispatchers(correlation_id: &str) {
    let _span = info_span!("Close Stream Writers").entered();
    let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
    if let Some(dispatchers) = &mut *dispatchers_lock {
        if *correlation_id != *dispatchers.correlation_id {
            debug! { "Owned by {} instead of {correlation_id}", dispatchers.correlation_id };
            return;
        }
        if let ShutdownPipe::Signal(signal) = dispatchers_lock
            .take()
            .or_throw("dispatchers_lock")
            .shutdown_pipe
        {
            match signal.send(()) {
                Ok(()) => info!("Closed"),
                Err(()) => debug!("Already shutdown"),
            }
        } else {
            warn!("Pipe was still pending")
        }
    }
}

/// Sends a request to close the pipe.
#[nameth]
pub async fn close_pipe(correlation_id: Arc<str>) {
    let span = info_span!("ClosePipe", %correlation_id);
    async {
        debug!("Start");
        defer!(debug!("End"));
        let _response = send_request(
            Method::POST,
            format!("{BASE_URL}/stream/{PIPE}/close"),
            set_headers(set_correlation_id(correlation_id.as_ref())),
        )
        .await
        .inspect_err(|error| warn!("Failed to close the pipe: {error}"));
    }
    .instrument(span)
    .await
}
