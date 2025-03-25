use terrazzo_pty::OpenProcessError;
use terrazzo_pty::ProcessIO;
use terrazzo_pty::lease::LeaseProcessOutputError;
use terrazzo_pty::lease::ProcessIoEntry;
use terrazzo_pty::lease::ProcessOutputLease;
use tracing::error;
use tracing::info;
use trz_gateway_server::server::Server;

use super::get_processes;
use crate::api::TerminalDef;
use crate::processes::io::PtyReader;
use crate::processes::io::PtyWriter;
use crate::terminal_id::TerminalId;

pub async fn open_stream<F>(
    _server: &Server,
    terminal_def: TerminalDef,
    open_process: impl Fn(&TerminalId) -> F,
) -> Result<ProcessOutputLease<PtyReader>, GetOrCreateProcessError>
where
    F: Future<Output = Result<ProcessIO, OpenProcessError>>,
{
    let processes = get_processes();
    let terminal_id = &terminal_def.id;
    match processes.entry(terminal_id.clone()) {
        dashmap::Entry::Occupied(occupied_entry) => {
            let entry = occupied_entry.get().1.clone();
            drop(occupied_entry);
            info!("Found");
            if let Ok(lease) = entry.lease_output().await {
                return Ok(lease);
            }
            info!("Can't get a lease");
            let process = open_process(terminal_id).await?;
            let process = process
                .map_input(PtyWriter::Local)
                .map_output(PtyReader::Local);
            let entry = ProcessIoEntry::new(process);
            processes.insert(terminal_id.clone(), (terminal_def, entry.clone()));
            return Ok(entry.lease_output().await?);
        }
        dashmap::Entry::Vacant(vacant_entry) => {
            info!("Not found");
            let process = open_process(terminal_id).await?;
            let process = process
                .map_input(PtyWriter::Local)
                .map_output(PtyReader::Local);
            let entry = ProcessIoEntry::new(process);
            vacant_entry.insert((terminal_def, entry.clone()));
            return Ok(entry.lease_output().await?);
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GetOrCreateProcessError {
    #[error("OpenProcessError: {0}")]
    OpenProcessError(#[from] OpenProcessError),

    #[error("LeaseProcessOutputError: {0}")]
    LeaseProcessOutputError(#[from] LeaseProcessOutputError),
}
