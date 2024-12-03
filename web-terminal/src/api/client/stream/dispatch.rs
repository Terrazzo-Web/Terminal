use futures::channel::mpsc::SendError;
use futures::SinkExt;
use named::named;
use named::NamedEnumValues as _;
use terrazzo::prelude::OrElseLog as _;
use tracing::trace;
use tracing::warn;

use super::DISPATCHERS;
use crate::api::Chunk;
use crate::api::NEWLINE;
use crate::terminal_id::TerminalId;

pub async fn dispatch(buffer: &mut Vec<u8>) {
    let mut consumed = 0;
    for chunk in buffer.split_inclusive(|c| *c == NEWLINE) {
        if chunk.last() == Some(&NEWLINE) {
            consumed += chunk.len();
            dispatch_chunk(&chunk[..chunk.len() - 1]).await;
        } else {
            break;
        }
    }
    buffer.drain(..consumed);
}

async fn dispatch_chunk(chunk: &[u8]) {
    if chunk.is_empty() {
        trace!("Received empty chunk"); // First chunk is empty.
        return;
    }
    let chunk: Chunk = match serde_json::from_slice(chunk) {
        Ok(chunk) => chunk,
        Err(error) => {
            warn!("Invalid chunk: {error}");
            return;
        }
    };

    if let Err(error) = send_chunk(&chunk.terminal_id, chunk.data).await {
        warn!("Failed to write chunk: {error}");
    }
}

async fn send_chunk(terminal_id: &TerminalId, chunk: Option<Vec<u8>>) -> Result<(), SendPartError> {
    let mut dispatcher = {
        let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
        let dispatchers = dispatchers_lock.as_mut().ok_or(SendPartError::NotFound)?;
        dispatchers
            .map
            .get_mut(terminal_id)
            .ok_or(SendPartError::NotFound)?
            .clone()
    };
    dispatcher
        .send(chunk)
        .await
        .map_err(SendPartError::SendError)
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum SendPartError {
    #[error("[{n}] Stream writer not registered", n = self.name())]
    NotFound,

    #[error("[{n}] Unable to send data through the channel: {0}", n = self.name())]
    SendError(SendError),
}
