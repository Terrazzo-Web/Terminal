use std::collections::HashMap;

use autoclone_macro::autoclone;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::FutureExt;
use futures::Stream;
use pin_project::pin_project;
use tracing::info;
use tracing::info_span;
use tracing::Instrument;
use web_sys::js_sys::Math;

use super::pipe::pipe;
use super::pipe::PipeError;
use super::register::register;
use super::register::RegisterError;
use super::StreamDispatchers;
use super::DISPATCHERS;
use crate::api::client::stream::close::drop_dispatcher;
use crate::api::client::stream::ShutdownPipe;
use crate::terminal_id::TerminalId;

pub async fn get(terminal_id: &TerminalId) -> Result<StreamReader, RegisterError> {
    async {
        let stream_reader = add_dispatcher(terminal_id).await?;
        register(terminal_id).await?;
        return Ok(stream_reader);
    }
    .instrument(info_span!("Get"))
    .await
}

async fn add_dispatcher(terminal_id: &TerminalId) -> Result<StreamReader, PipeError> {
    let (tx, rx) = mpsc::channel(10);
    let (pipe_tx, pipe_rx) = oneshot::channel();
    add_dispatcher_sync(terminal_id, tx, pipe_tx);
    let () = pipe_rx.await.unwrap_or_else(|_| Err(PipeError::Canceled))?;
    Ok(StreamReader {
        id: OnStreamReaderDrop(terminal_id.clone()),
        rx,
    })
}

#[autoclone]
fn add_dispatcher_sync(
    terminal_id: &TerminalId,
    tx: mpsc::Sender<Vec<u8>>,
    pipe_tx: oneshot::Sender<Result<(), PipeError>>,
) {
    let mut dispatchers_lock = DISPATCHERS.lock().unwrap();
    let dispatchers = if let Some(dispatchers) = &mut *dispatchers_lock {
        info!("Use current dispatchers");
        match &dispatchers.shutdown_pipe {
            ShutdownPipe::Pending(shared) => wasm_bindgen_futures::spawn_local(async move {
                autoclone!(shared);
                match shared.clone().await {
                    Ok(()) => {
                        let _ = pipe_tx.send(Ok(()));
                    }
                    Err(oneshot::Canceled) => {
                        let _ = pipe_tx.send(Err(PipeError::Canceled));
                    }
                }
            }),
            ShutdownPipe::Signal { .. } => {
                let _ = pipe_tx.send(Ok(()));
            }
        }
        dispatchers
    } else {
        info!("Allocate new dispatchers");
        let correlation_id = format!("{:#x}", Math::random().to_bits() % 22633363);
        let (pending_tx, pending_rx) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            autoclone!(correlation_id);
            let shutdown_pipe = match pipe(&correlation_id).await {
                Ok(shutdown_pipe) => shutdown_pipe,
                Err(error) => {
                    let _ = pipe_tx.send(Err(error));
                    *DISPATCHERS.lock().unwrap() = None;
                    return;
                }
            };
            if let Some(dispatchers) = &mut *DISPATCHERS.lock().unwrap() {
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
        dispatchers_lock.as_mut().unwrap()
    };
    dispatchers.map.insert(terminal_id.clone(), tx);
}

// The reader contains the reading part of the dispatcher.
// On drop it removes the dispatcher.
#[pin_project]
pub struct StreamReader {
    id: OnStreamReaderDrop,
    #[pin]
    rx: mpsc::Receiver<Vec<u8>>,
}

struct OnStreamReaderDrop(TerminalId);

impl Stream for StreamReader {
    type Item = Vec<u8>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().rx.poll_next(cx)
    }
}

impl Drop for OnStreamReaderDrop {
    fn drop(&mut self) {
        drop_dispatcher(&self.0);
    }
}
