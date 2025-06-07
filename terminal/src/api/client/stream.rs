use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::Shared;
use get::TerminalStream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::prelude::OrElseLog as _;
use tracing::debug;
use tracing::info;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::Closure;
use web_sys::Element;
use web_sys::MouseEvent;
use web_sys::js_sys::Uint8Array;

use self::get::get;
use self::register::RegisterError;
use super::request::SendRequestError;
use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalRequest;
use crate::api::TerminalDef;
use crate::terminal::TerminalsState;
use crate::terminal_id::TerminalId;

mod ack;
mod close;
mod dispatch;
mod get;
mod keepalive;
mod pipe;
mod register;

pub use self::close::close;
pub use self::close::drop_dispatcher;
pub use self::pipe::close_pipe;

static GLOBAL_AWAKE: Mutex<Option<(oneshot::Sender<()>, Shared<oneshot::Receiver<()>>)>> =
    Mutex::new(None);

/// Pumps data into XTermJS.
///
/// EOS flow:
/// 1. Stream ends or fails
/// 2. [Close the terminal](TerminalsState::close_terminal)
/// 3. Tab is removed from UI
/// 4. [Close the stream](fn@crate::api::client::stream::close)
///    - Take the dispatcher out of the map
///    - If the map is empty, add the correlation id
/// 5. */Server side/* Close the process
/// 6. */Server side/* If there is a correlation id, drop the registration
pub async fn stream<F, F0>(
    state: TerminalsState,
    terminal_def: TerminalDef,
    element: Element,
    on_init: impl FnOnce() -> F0,
    on_data: impl Fn(JsValue) -> F,
) -> Result<(), StreamError>
where
    F: Future<Output = ()>,
    F0: Future<Output = ()>,
{
    let terminal_id = terminal_def.address.id.clone();
    defer! { state.on_eos(&terminal_id); }
    let query = RegisterTerminalRequest {
        mode: RegisterTerminalMode::Create,
        def: terminal_def,
    };
    let mut reader = get(query.clone()).await?;

    debug!("On init");
    let () = on_init().await;

    debug!("Streaming");
    loop {
        match do_stream(reader, &on_data).await {
            StreamStatus::PipeDisconnected => (),
            StreamStatus::EndOfStream => return Ok(()),
        };
        info!("Streaming stopped");
        let streaming_state = Rc::new(Cell::new(None));

        let closure = make_wake_closure(element.clone(), streaming_state.clone());
        let () = element
            .add_event_listener_with_callback(WAKE_EVENT_TYPE, closure.as_ref().unchecked_ref())
            .unwrap_or_else(|error| warn!("Unable to attach mouse move event handler: {error:?}"));
        let rx = {
            let mut global_awake = GLOBAL_AWAKE.lock().or_throw("GLOBAL_AWAKE");
            match &*global_awake {
                Some((_tx, rx)) => rx.clone(),
                None => {
                    let (tx, rx) = oneshot::channel();
                    let rx = rx.shared();
                    *global_awake = Some((tx, rx.clone()));
                    rx
                }
            }
        };
        streaming_state.set(Some(closure));

        match rx.await {
            Ok(()) => debug!("Wake-up to continue streaming"),
            Err(oneshot::Canceled) => {
                debug!("Not restarting stream, terminal is canceled");
                return Ok(());
            }
        }
        let query = RegisterTerminalRequest {
            mode: RegisterTerminalMode::Reopen,
            def: query.def.clone(),
        };
        let Ok(new_reader) = get(query).await else {
            warn!("Can't re-open the stream");
            return Ok(());
        };
        info!("Re-opened the stream");
        reader = new_reader;
    }
}

async fn do_stream<F>(
    mut reader: impl TerminalStream,
    on_data: &impl Fn(JsValue) -> F,
) -> StreamStatus
where
    F: Future<Output = ()>,
{
    while let Some(chunks) = reader.next().await {
        let chunk = {
            let capacity = chunks
                .iter()
                .filter_map(|chunk| chunk.as_ref().map(Vec::len))
                .sum();
            let mut concat = Vec::with_capacity(capacity);
            for chunk in chunks {
                let Some(chunk) = chunk else {
                    debug!("End of stream");
                    return StreamStatus::EndOfStream;
                };
                concat.extend_from_slice(&chunk);
            }
            concat
        };
        let js_value = Uint8Array::new_with_length(chunk.len() as u32);
        js_value.copy_from(&chunk);
        let () = on_data(js_value.into()).await;
    }
    return StreamStatus::PipeDisconnected;
}

#[must_use]
enum StreamStatus {
    PipeDisconnected,
    EndOfStream,
}

const WAKE_EVENT_TYPE: &str = "mousemove";

fn make_wake_closure(
    element: Element,
    closure: Rc<Cell<Option<Closure<dyn Fn(MouseEvent)>>>>,
) -> Closure<dyn Fn(MouseEvent)> {
    Closure::new(move |_| {
        if let Some(closure) = closure.take() {
            debug!("Mouse move triggers restart stream");
            let function = closure.as_ref().unchecked_ref();
            let () = element
                .remove_event_listener_with_callback(WAKE_EVENT_TYPE, function)
                .unwrap_or_else(|error| warn!("Failed to remove event handler: {error:?}"));
            try_restart_pipe();
        } else {
            warn!("Event handler fired twice");
        }
    })
}

pub fn try_restart_pipe() {
    let Some((tx, _rx)) = GLOBAL_AWAKE.lock().or_throw("GLOBAL_AWAKE").take() else {
        return;
    };
    let _ = tx.send(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    RegisterError(#[from] RegisterError),
}

struct StreamDispatchers {
    correlation_id: Arc<str>,
    map: HashMap<TerminalId, mpsc::Sender<Option<Vec<u8>>>>,
    shutdown_pipe: ShutdownPipe,
}

enum ShutdownPipe {
    Pending(Shared<oneshot::Receiver<()>>),
    Signal(oneshot::Sender<()>),
}

static DISPATCHERS: Dispatchers = Dispatchers::new();

struct Dispatchers(Mutex<Option<StreamDispatchers>>);

impl Dispatchers {
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }
}

impl Deref for Dispatchers {
    type Target = Mutex<Option<StreamDispatchers>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
