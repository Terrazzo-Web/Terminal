use std::collections::HashMap;

use futures::SinkExt as _;
use futures::Stream;
use futures::StreamExt as _;
use futures::channel::mpsc;
use scopeguard::defer;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use super::Dispatchers;
use super::DispatchersInner;
use super::lock::DispatchersLock;
use crate::api::Chunk;
use crate::api::WriteRequest;
use crate::api::client::channel;
use crate::api::client::channel::WebChannelError;
use crate::api::client::channel::download::DownloadItemError;
use crate::api::client::request::BASE_URL;
use crate::terminal_id::TerminalId;

impl Dispatchers {
    pub async fn get_or_init(&'static self) -> Result<DispatchersLock<'static>, WebChannelError> {
        {
            // Return current dispatchers if available
            let lock = self.lock();
            if lock.is_some() {
                return Ok(DispatchersLock::new(lock));
            }
        }

        let mut inner = {
            // The channel is not connected: open it.
            let (upload, download) = open().await?;
            spawn_local(dispatch_download(self, download));
            DispatchersInner {
                download: HashMap::default(),
                upload,
            }
        };

        {
            // Return current dispatchers if available
            let mut lock = self.lock();
            if lock.is_some() {
                return Err(WebChannelError::Race);
            }
            *lock = Some(inner);
            return Ok(DispatchersLock::new(lock));
        }
    }
}

/// Opens a full-duplex channel with the backend.
async fn open() -> Result<
    (
        mpsc::Sender<WriteRequest>,
        impl Stream<Item = Result<Chunk, DownloadItemError>> + Unpin,
    ),
    WebChannelError,
> {
    let upload = mpsc::channel(0);
    let download = channel::open_channel(
        &format!("{BASE_URL}/stream"),
        |_on_upload_request| {},
        || |_on_download_request| {},
        upload.1,
    )
    .await?;
    Ok((upload.0, download))
}

/// Reads from the download stream and dispatch chunks to each terminal's own stream.
async fn dispatch_download(
    dispatchers: &'static Dispatchers,
    mut download: impl Stream<Item = Result<Chunk, DownloadItemError>> + Unpin,
) {
    defer!(*dispatchers.lock() = None);
    let mut download_cache = HashMap::default();
    while let Some(next) = download.next().await {
        let (terminal_id, data) = match next {
            Ok(Chunk { terminal_id, data }) => (terminal_id, data),
            Err(error) => return warn!("Stream failed: {error}"),
        };

        let Some(mut terminal) =
            get_terminal_download(dispatchers, &mut download_cache, &terminal_id)
        else {
            continue;
        };

        let Some(data) = data else {
            close_terminal_download(dispatchers, &mut download_cache, &terminal_id);
            continue;
        };

        let Ok(()) = terminal.send(data).await else {
            download_cache.remove(&terminal_id);
            let mut lock = dispatchers.lock();
            let Some(dispatchers) = lock.as_mut() else {
                return;
            };
            dispatchers.download.remove(&terminal_id);
            if dispatchers.download.is_empty() {
                return;
            }
            continue;
        };
    }
}

/// Get and cache a terminal's download channel.
fn get_terminal_download<'t>(
    dispatchers: &'static Dispatchers,
    download_cache: &'t mut HashMap<TerminalId, mpsc::Sender<Vec<u8>>>,
    terminal_id: &TerminalId,
) -> Option<&'t mut mpsc::Sender<Vec<u8>>> {
    if let Some(terminal) = download_cache.get(terminal_id) {
        Some(terminal);
    }
    if let Some(inner) = &mut *dispatchers.lock() {
        if let Some(terminal) = inner.download.get(terminal_id) {
            let entry = download_cache
                .entry(terminal_id.clone())
                .insert_entry(terminal.clone());
            return Some(entry.into_mut());
        }
    }
    return None;
}

/// Close a terminal's download channel.
fn close_terminal_download<'t>(
    dispatchers: &'static Dispatchers,
    download_cache: &'t mut HashMap<TerminalId, mpsc::Sender<Vec<u8>>>,
    terminal_id: &TerminalId,
) {
    download_cache.remove(terminal_id);
    let terminal = if let Some(inner) = &mut *dispatchers.lock() {
        inner.download.remove(terminal_id)
    } else {
        return;
    };
    drop(terminal);
}
