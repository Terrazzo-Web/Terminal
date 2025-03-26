use futures::channel::mpsc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
use tracing::debug;
use tracing::warn;
use tracing_futures as _;
use trz_gateway_server::server::Server;

use super::registration::Registration;
use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalRequest;
use crate::processes;
use crate::processes::io::HybridReader;
use crate::processes::io::LocalReader;
use crate::terminal_id::TerminalId;

pub async fn register(
    server: &Server,
    request: RegisterTerminalRequest,
) -> Result<(), RegisterStreamError> {
    defer!(debug!("End"));
    debug!("Start");
    async {
        let terminal_id = request.def.id.clone();
        let stream = processes::stream::open_stream(server, request.def, |_| async {
            match request.mode {
                // TODO: if it's a remote terminal, open a ProcessIO connected to a remote client
                RegisterTerminalMode::Create => ProcessIO::open().await,
                RegisterTerminalMode::Reopen => Err(OpenProcessError::NotFound),
            }
        })
        .await?;
        let stream = LocalReader(HybridReader::Local(stream));
        push_lease(terminal_id, stream)?;
        Ok(())
    }
    .await
    .inspect_err(|err| warn!("{err}"))
}

fn push_lease(terminal_id: TerminalId, stream: LocalReader) -> Result<(), PushLeaseError> {
    #[cfg(debug_assertions)]
    let stream = tracing_futures::Instrument::instrument(stream, tracing::debug_span!("Lease"));

    Ok(Registration::current()
        .ok_or(PushLeaseError::NoClientRegisteredError)?
        .try_send((terminal_id, stream))
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PushLeaseError {
    #[error("[{n}] Expected a client to be registered", n = self.name())]
    NoClientRegisteredError,

    #[error("[{n}] Failed to send lease: {0}", n = self.name())]
    SendError(#[from] mpsc::SendError),
}
