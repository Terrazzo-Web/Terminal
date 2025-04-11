use futures::StreamExt;
use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Uint8Array;

use super::dispatcher::DISPATCHERS;
use crate::api::TerminalAddress;
use crate::api::client::channel::WebChannelError;

static DISPATCHER_BUFFER_SIZE: usize = 10;

pub async fn read<F>(
    terminal: &TerminalAddress,
    on_data: impl Fn(JsValue) -> F,
) -> Result<(), ReadStreamError>
where
    F: Future<Output = ()>,
{
    let mut rx = {
        let mut dispatchers = DISPATCHERS.get_or_init().await?;
        let (tx, rx) = mpsc::channel(DISPATCHER_BUFFER_SIZE);
        dispatchers.download.insert(terminal.id.clone(), tx);
        rx
    };

    while let Some(chunk) = rx.next().await {
        let js_value = Uint8Array::new_with_length(chunk.len() as u32);
        js_value.copy_from(&chunk);
        let () = on_data(js_value.into()).await;
    }
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ReadStreamError {
    #[error("[{n}] {0}", n = self.name())]
    WebChannel(#[from] WebChannelError),
}
