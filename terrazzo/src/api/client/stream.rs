use std::collections::HashMap;
use std::sync::Mutex;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::Shared;
use futures::StreamExt as _;
use named::named;
use named::NamedEnumValues as _;
use tracing::debug;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;

use self::get::get;
use self::register::RegisterError;
use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use crate::terminal_id::TerminalId;

mod close;
mod dispatch;
mod get;
mod pipe;
mod register;

pub use self::close::close;

pub async fn stream<F, F0>(
    terminal_id: &TerminalId,
    on_init: impl FnOnce() -> F0,
    on_data: impl Fn(JsValue) -> F,
) -> Result<(), StreamError>
where
    F: std::future::Future<Output = ()>,
    F0: std::future::Future<Output = ()>,
{
    let mut reader = get(terminal_id).await?.ready_chunks(10);
    debug!("On init");
    let () = on_init().await;
    debug!("Streaming");
    while let Some(chunks) = reader.next().await {
        let chunk = chunks.concat();
        let js_value = Uint8Array::new_with_length(chunk.len() as u32);
        js_value.copy_from(&chunk);
        let () = on_data(js_value.into()).await;
    }
    Ok(())
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{}] {0}", self.name())]
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
