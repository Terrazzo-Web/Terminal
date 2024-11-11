use futures::channel::mpsc;
use named::named;
use named::NamedEnumValues as _;
use scopeguard::defer;
use terrazzo_pty::lease::ProcessOutputLease;
use terrazzo_pty::ProcessIO;
use tracing::debug;
use tracing::warn;
use tracing_futures as _;

use super::registration::Registration;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn register(terminal_id: TerminalId) -> Result<(), RegisterStreamError> {
    register_impl(terminal_id).await
}

async fn register_impl(terminal_id: TerminalId) -> Result<(), RegisterStreamError> {
    defer!(debug!("End"));
    debug!("Start");
    async {
        let lease = processes::stream::open_stream(&terminal_id, |_| ProcessIO::open()).await?;
        push_lease(terminal_id, lease)?;
        Ok(())
    }
    .await
    .inspect_err(|err| warn!("{err}"))
}

fn push_lease(terminal_id: TerminalId, lease: ProcessOutputLease) -> Result<(), PushLeaseError> {
    #[cfg(debug_assertions)]
    let lease = tracing_futures::Instrument::instrument(lease, tracing::debug_span!("Lease"));

    Ok(Registration::current()
        .ok_or(PushLeaseError::NoClientRegisteredError)?
        .try_send((terminal_id, lease))
        .map_err(|err| err.into_send_error())?)
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum RegisterStreamError {
    #[error("[{n}] {0}", n = self.name())]
    GetOrCreateProcessError(#[from] processes::stream::GetOrCreateProcessError),

    #[error("[{n}] {0}", n = self.name())]
    PushLeaseError(#[from] PushLeaseError),
}

#[derive(thiserror::Error, Debug)]
pub enum PushLeaseError {
    #[error("NoClientRegisteredError")]
    NoClientRegisteredError,

    #[error("SendError: {0}")]
    SendError(#[from] mpsc::SendError),
}
