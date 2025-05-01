use futures::SinkExt;
use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;

use super::dispatcher::DISPATCHERS;
use crate::api::TerminalAddress;
use crate::api::WriteRequest;
use crate::api::client::channel::WebChannelError;

pub async fn write(terminal: TerminalAddress, data: String) -> Result<(), WriteError> {
    let mut dispatchers = DISPATCHERS.get_or_init().await?;
    Ok(dispatchers
        .upload
        .send(WriteRequest { terminal, data })
        .await?)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{n}] {0}", n = self.name())]
    WebChannel(#[from] WebChannelError),

    #[error("[{n}] Failed to send: {0}", n = self.name())]
    SendError(#[from] mpsc::SendError),
}
