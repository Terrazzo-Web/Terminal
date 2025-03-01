use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
use terrazzo_pty::lease::ProcessOutputLease;
use tracing::debug;
use tracing::warn;
use tracing_futures as _;

use super::registration::Registration;
use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalRequest;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn register(request: RegisterTerminalRequest) -> Result<(), RegisterStreamError> {
    defer!(debug!("End"));
    debug!("Start");
    async {
        let terminal_id = request.def.id.clone();
        let lease = processes::stream::open_stream(request.def, |_| async {
            match request.mode {
                RegisterTerminalMode::Create => ProcessIO::open().await,
                RegisterTerminalMode::Reopen => Err(OpenProcessError::NotFound),
            }
        })
        .await?;
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

#[nameth]
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
