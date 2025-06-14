use std::collections::HashMap;
use std::sync::Arc;

use futures::FutureExt;
use futures::Stream;
use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::channel::oneshot;
use pin_project::pin_project;
use terrazzo::autoclone;
use terrazzo::declare_trait_aliias;
use terrazzo::prelude::OrElseLog as _;
use tracing::info;
use tracing::info_span;
use tracing_futures::Instrument as _;
use web_sys::js_sys::Math;

use super::DISPATCHERS;
use super::StreamDispatchers;
use super::ack;
use super::pipe::PipeError;
use super::pipe::pipe;
use super::register::RegisterError;
use super::register::register;
use crate::api::RegisterTerminalRequest;
use crate::api::client::stream::ShutdownPipe;
use crate::terminal_id::TerminalId;

declare_trait_aliias!(TerminalStream, Stream<Item = Vec<Option<Vec<u8>>>> + Unpin);

pub async fn get(request: RegisterTerminalRequest) -> Result<impl TerminalStream, RegisterError> {
    let span = info_span!("Get", terminal_id = %request.def.address.id);
    async {
        let stream_reader = add_dispatcher(&request.def.address.id).await?;
        let stream_reader = ack::setup_acks(request.def.address.clone(), stream_reader);
        register(request).await?;
        return Ok(stream_reader.ready_chunks(10).in_current_span());
    }
    .instrument(span)
    .await
}

async fn add_dispatcher(terminal_id: &TerminalId) -> Result<StreamReader, PipeError> {
    let (tx, rx) = mpsc::channel(10);
    let (pipe_tx, pipe_rx) = oneshot::channel();
    add_dispatcher_sync(terminal_id, tx, pipe_tx);
    let () = pipe_rx.await.unwrap_or_else(|_| Err(PipeError::Canceled))?;
    Ok(StreamReader { rx })
}

#[autoclone]
fn add_dispatcher_sync(
    terminal_id: &TerminalId,
    tx: mpsc::Sender<Option<Vec<u8>>>,
    pipe_tx: oneshot::Sender<Result<(), PipeError>>,
) {
    let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
    let dispatchers = if let Some(dispatchers) = &mut *dispatchers_lock {
        info!("Use current dispatchers");
        match &dispatchers.shutdown_pipe {
            ShutdownPipe::Pending(shared) => wasm_bindgen_futures::spawn_local(async move {
                autoclone!(shared);
                let pipe_status = shared
                    .await
                    .map_err(|oneshot::Canceled| PipeError::Canceled);
                let _ = pipe_tx.send(pipe_status);
            }),
            ShutdownPipe::Signal { .. } => {
                let _ = pipe_tx.send(Ok(()));
            }
        }
        dispatchers
    } else {
        info!("Allocate new dispatchers");
        let correlation_id: Arc<str> = format!("{:#x}", Math::random().to_bits() % 22633363).into();
        let (pending_tx, pending_rx) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            autoclone!(correlation_id);
            let shutdown_pipe = match pipe(correlation_id).await {
                Ok(shutdown_pipe) => shutdown_pipe,
                Err(error) => {
                    let _ = pipe_tx.send(Err(error));
                    *DISPATCHERS.lock().or_throw("DISPATCHERS") = None;
                    return;
                }
            };
            if let Some(dispatchers) = &mut *DISPATCHERS.lock().or_throw("DISPATCHERS") {
                dispatchers.shutdown_pipe = ShutdownPipe::Signal(shutdown_pipe);
            }
            let _ = pipe_tx.send(Ok(()));
            let _ = pending_tx.send(());
        });
        *dispatchers_lock = Some(StreamDispatchers {
            correlation_id,
            map: HashMap::new(),
            shutdown_pipe: ShutdownPipe::Pending(pending_rx.shared()),
        });
        dispatchers_lock.as_mut().or_throw("dispatchers_lock")
    };
    dispatchers.map.insert(terminal_id.clone(), tx);
}

// The reader contains the reading part of the dispatcher.
// On drop it removes the dispatcher.
#[pin_project]
pub struct StreamReader {
    #[pin]
    rx: mpsc::Receiver<Option<Vec<u8>>>,
}

impl Stream for StreamReader {
    type Item = Option<Vec<u8>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().rx.poll_next(cx)
    }
}
