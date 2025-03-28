use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryFutureExt as _;
use futures::channel::oneshot;
use futures::select;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::prelude::OrElseLog as _;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Headers;
use web_sys::Response;
use web_sys::js_sys::Uint8Array;

use super::BASE_URL;
use super::DISPATCHERS;
use super::Method;
use super::SendRequestError;
use super::ShutdownPipe;
use super::dispatch::dispatch;
use super::send_request;
use crate::api::CORRELATION_ID;

/// Spawns the pipe in the background.
#[nameth]
pub async fn pipe(correlation_id: &str) -> Result<oneshot::Sender<()>, PipeError> {
    async move {
        info!("Start");
        let response = send_request(
            Method::POST,
            format!("{BASE_URL}/stream/{PIPE}"),
            move |request| {
                let headers = Headers::new().or_throw("Headers::new()");
                headers
                    .set(CORRELATION_ID, correlation_id)
                    .or_throw(CORRELATION_ID);
                request.set_headers(headers.as_ref());
            },
        )
        .await?;
        let Some(stream) = response.body() else {
            return Err(PipeError::EmptyStream);
        };

        info!("Streaming");
        let (tx, rx) = oneshot::channel();
        let correlation_id = correlation_id.to_owned();
        let streaming_task = async move {
            // Close all the stream dispatchers if the pipe fails.
            defer! { close_dispatchers(&correlation_id); };
            if let Err(error) = pipe_impl(rx, stream).await {
                warn!("Pipe failed: {error}");
            }
            info!("Closed");
        };
        spawn_local(streaming_task.in_current_span());
        return Ok(tx);
    }
    .instrument(info_span!("Pipe"))
    .await
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
            buffer.extend(std::iter::repeat(b'\0').take(count));
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
}

fn close_dispatchers(correlation_id: &str) {
    let _span = info_span!("Close Stream Writers").entered();
    let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
    if let Some(dispatchers) = &mut *dispatchers_lock {
        if *correlation_id != dispatchers.correlation_id {
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
pub fn close_pipe(correlation_id: String) -> impl Future<Output = ()> {
    let span = info_span!("ClosePipe", %correlation_id);
    send_request(
        Method::POST,
        format!("{BASE_URL}/stream/{PIPE}/close"),
        move |request| {
            debug!("Start");
            let headers = Headers::new().or_throw("Headers::new()");
            headers
                .set(CORRELATION_ID, &correlation_id)
                .or_throw(CORRELATION_ID);
            request.set_headers(headers.as_ref());
        },
    )
    .map(|response| {
        debug!("End");
        let _: Response = response?;
        Ok(())
    })
    .unwrap_or_else(|error: PipeError| warn!("Failed to close the pipe: {error}"))
    .instrument(span)
}
