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
use terrazzo::prelude::diagnostics;
use wasm_bindgen_futures::spawn_local;
use web_sys::js_sys::Math;

use self::diagnostics::Instrument as _;
use self::diagnostics::info;
use self::diagnostics::info_span;
use super::DISPATCHERS;
use super::ShutdownPipe;
use super::StreamDispatchers;
use super::ack;
use super::pipe;
use super::register;
use crate::api::shared::terminal_schema::RegisterTerminalRequest;
use crate::terminal_id::TerminalId;

declare_trait_aliias!(TerminalStream, Stream<Item = Vec<Option<Vec<u8>>>> + Unpin);

pub async fn get(
    request: RegisterTerminalRequest,
) -> Result<impl TerminalStream, register::RegisterError> {
    let span = info_span!("Get", terminal_id = %request.def.address.id);
    async {
        let stream_reader = add_dispatcher(&request.def.address.id).await?;
        let stream_reader = ack::setup_acks(request.def.address.clone(), stream_reader);
        register::register(request).await?;
        return Ok(stream_reader.ready_chunks(10));
    }
    .instrument(span)
    .await
}

async fn add_dispatcher(terminal_id: &TerminalId) -> Result<StreamReader, pipe::PipeError> {
    let (tx, rx) = mpsc::channel(10);
    let (pipe_tx, pipe_rx) = oneshot::channel();
    add_dispatcher_sync(terminal_id, tx, pipe_tx);
    let () = pipe_rx
        .await
        .unwrap_or_else(|_| Err(pipe::PipeError::Canceled))?;
    Ok(StreamReader { rx })
}

#[autoclone]
fn add_dispatcher_sync(
    terminal_id: &TerminalId,
    tx: mpsc::Sender<Option<Vec<u8>>>,
    pipe_tx: oneshot::Sender<Result<(), pipe::PipeError>>,
) {
    let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
    let dispatchers = if let Some(dispatchers) = &mut *dispatchers_lock {
        info!("Use current dispatchers");
        match &dispatchers.shutdown_pipe {
            ShutdownPipe::Pending(shared) => {
                let shutdown_pipe = async move {
                    autoclone!(shared);
                    let pipe_status = shared
                        .await
                        .map_err(|oneshot::Canceled| pipe::PipeError::Canceled);
                    let _ = pipe_tx.send(pipe_status);
                };
                spawn_local(shutdown_pipe.in_current_span())
            }
            ShutdownPipe::Signal { .. } => {
                let _ = pipe_tx.send(Ok(()));
            }
        }
        dispatchers
    } else {
        info!("Allocate new dispatchers");
        let correlation_id: Arc<str> = format!("{:#x}", Math::random().to_bits() % 22633363).into();
        let (pending_tx, pending_rx) = oneshot::channel();
        let allocate_dispatchers_task = async move {
            autoclone!(correlation_id);
            let shutdown_pipe = match pipe::pipe(correlation_id).await {
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
        };
        spawn_local(allocate_dispatchers_task.in_current_span());
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
