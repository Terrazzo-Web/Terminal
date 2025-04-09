#![allow(unused)]

use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use futures::Sink;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tracing::warn;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Element;
use web_sys::js_sys::Uint8Array;

use super::channel::WebChannelError;
use super::channel::download::DownloadItemError;
use super::channel::upload;
use super::request::BASE_URL;
use crate::api::Chunk;
use crate::api::TerminalAddress;
use crate::api::TerminalDef;
use crate::api::WriteRequest;
use crate::api::client::channel::download::get_download_stream;
use crate::terminal::TerminalsState;
use crate::terminal_id::TerminalId;

mod init;
mod lock;

pub async fn stream<F>(
    terminal_def: TerminalDef,
    on_data: impl Fn(JsValue) -> F,
) -> Result<(), StreamError>
where
    F: Future<Output = ()>,
{
    let mut rx = {
        let mut dispatchers = DISPATCHERS.get_or_init().await?;
        let (tx, rx) = mpsc::channel(DISPATCHER_BUFFER_SIZE);
        dispatchers
            .download
            .insert(terminal_def.address.id.clone(), tx);
        rx
    };

    while let Some(chunk) = rx.next().await {
        let js_value = Uint8Array::new_with_length(chunk.len() as u32);
        js_value.copy_from(&chunk);
        let () = on_data(js_value.into()).await;
    }
    return Ok(());
}

pub async fn close(terminal: &TerminalAddress) {
    let mut dispatchers = DISPATCHERS.lock();
    let Some(dispatchers) = &mut *dispatchers else {
        return;
    };
    let Some(mut dispatcher) = dispatchers.download.remove(&terminal.id) else {
        return;
    };
    dispatcher.close_channel();
}

static DISPATCHER_BUFFER_SIZE: usize = 10;

static DISPATCHERS: Dispatchers = Dispatchers(Mutex::new(None));

struct Dispatchers(std::sync::Mutex<Option<DispatchersInner>>);

pub struct DispatchersInner {
    download: HashMap<TerminalId, mpsc::Sender<Vec<u8>>>,
    upload: mpsc::Sender<WriteRequest>,
}

impl Dispatchers {
    fn lock(&self) -> MutexGuard<'_, Option<DispatchersInner>> {
        self.0.lock().unwrap()
    }
}

unsafe impl Sync for Dispatchers {}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("[{n}] {0}", n = self.name())]
    WebChannel(#[from] WebChannelError),
}
