use futures::channel::oneshot;
use futures::select;
use futures::StreamExt;
use named::named;
use named::NamedEnumValues as _;
use scopeguard::defer;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use tracing::Instrument;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::js_sys::Uint8Array;
use web_sys::Headers;

use super::dispatch::dispatch;
use super::send_request;
use super::Method;
use super::SendRequestError;
use super::ShutdownPipe;
use super::BASE_URL;
use super::DISPATCHERS;
use crate::api::CORRELATION_ID;

/// Spawns the pipe in the background.
#[named]
pub async fn pipe(correlation_id: &str) -> Result<oneshot::Sender<()>, PipeError> {
    async move {
        info!("Start");
        let response = send_request(
            Method::GET,
            format!("{BASE_URL}/stream/{PIPE}"),
            move |request| {
                let headers = Headers::new().expect("Headers::new()");
                headers
                    .set(CORRELATION_ID, correlation_id)
                    .expect(CORRELATION_ID);
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

#[named]
#[derive(thiserror::Error, Debug)]
pub enum PipeError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{}] Pipe is an empty stream", self.name())]
    EmptyStream,

    #[error("[{}] Chunk is not a byte array: {0:?}", self.name())]
    InvalidChunk(JsValue),

    #[error("[{}] Stream failed: {0:?}", self.name())]
    ReadError(JsValue),

    #[error("[{}] Pipe canceled", self.name())]
    Canceled,
}

fn close_dispatchers(correlation_id: &str) {
    let _span = info_span!("Close Stream Writers").entered();
    let mut dispatchers_lock = DISPATCHERS.lock().unwrap();
    if let Some(dispatchers) = &mut *dispatchers_lock {
        if *correlation_id != dispatchers.correlation_id {
            debug! { "Owned by {} instead of {correlation_id}", dispatchers.correlation_id };
            return;
        }
        if let ShutdownPipe::Signal(signal) = dispatchers_lock.take().unwrap().shutdown_pipe {
            match signal.send(()) {
                Ok(()) => info!("Closed"),
                Err(()) => debug!("Already shutdown"),
            }
        } else {
            warn!("Pipe was still pending")
        }
    } else {
        info!("Not set")
    }
}
