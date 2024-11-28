use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use autoclone_macro::autoclone;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::Shared;
use futures::stream::ReadyChunks;
use futures::StreamExt as _;
use get::StreamReader;
use named::named;
use named::NamedEnumValues as _;
use tracing::debug;
use tracing::info;
use tracing::warn;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;
use web_sys::Element;
use web_sys::MouseEvent;

use self::get::get;
use self::register::RegisterError;
use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalQuery;
use crate::terminal_id::TerminalId;

mod close;
mod dispatch;
mod get;
mod pipe;
mod register;

pub use self::close::close;

#[autoclone]
pub async fn stream<F, F0>(
    terminal_id: &TerminalId,
    element: Element,
    on_init: impl FnOnce() -> F0,
    on_data: impl Fn(JsValue) -> F,
) -> Result<(), StreamError>
where
    F: std::future::Future<Output = ()>,
    F0: std::future::Future<Output = ()>,
{
    let mut reader = get(
        terminal_id,
        RegisterTerminalQuery {
            mode: RegisterTerminalMode::Create,
        },
    )
    .await?
    .ready_chunks(10);
    debug!("On init");
    let () = on_init().await;
    debug!("Streaming");
    loop {
        do_stream(reader, &on_data).await;
        info!("Streaming stopped");
        let closure_state: Rc<Cell<Option<(oneshot::Sender<()>, Closure<dyn Fn(MouseEvent)>)>>> =
            Rc::new(Cell::new(None));
        const WAKE_EVENT_TYPE: &str = "mousemove";
        let closure: Closure<dyn Fn(MouseEvent)> = Closure::new(move |_| {
            autoclone!(element, closure_state);
            if let Some((tx, closure)) = closure_state.take() {
                debug!("Mouse move triggers restart stream");
                let function = closure.as_ref().unchecked_ref();
                let () = element
                    .remove_event_listener_with_callback(WAKE_EVENT_TYPE, function)
                    .unwrap_or_else(|error| warn!("Failed to remove event handler: {error:?}"));
                let _ = tx.send(());
            } else {
                warn!("Event handler fired twice");
            }
        });
        let () = element
            .add_event_listener_with_callback(WAKE_EVENT_TYPE, closure.as_ref().unchecked_ref())
            .unwrap_or_else(|error| warn!("Unable to attach mouse move event handler: {error:?}"));
        let (tx, rx) = oneshot::channel();
        closure_state.set(Some((tx, closure)));

        match rx.await {
            Ok(()) => debug!("Restarting stream"),
            Err(oneshot::Canceled) => {
                debug!("Not restarting stream, terminal is canceled");
                return Ok(());
            }
        }
        let Ok(new_reader) = get(
            terminal_id,
            RegisterTerminalQuery {
                mode: RegisterTerminalMode::Reopen,
            },
        )
        .await
        .map(|reader| reader.ready_chunks(10)) else {
            info!("The process was stopped");
            return Ok(());
        };
        info!("Re-opened the stream");
        reader = new_reader;
    }
}

async fn do_stream<F>(mut reader: ReadyChunks<StreamReader>, on_data: &impl Fn(JsValue) -> F)
where
    F: std::future::Future<Output = ()>,
{
    while let Some(chunks) = reader.next().await {
        let chunk = chunks.concat();
        let js_value = Uint8Array::new_with_length(chunk.len() as u32);
        js_value.copy_from(&chunk);
        let () = on_data(js_value.into()).await;
    }
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    RegisterError(#[from] RegisterError),
}

struct StreamDispatchers {
    correlation_id: String,
    map: HashMap<TerminalId, mpsc::Sender<Vec<u8>>>,
    shutdown_pipe: ShutdownPipe,
}

enum ShutdownPipe {
    Pending(Shared<oneshot::Receiver<()>>),
    Signal(oneshot::Sender<()>),
}

static DISPATCHERS: Mutex<Option<StreamDispatchers>> = Mutex::new(None);
