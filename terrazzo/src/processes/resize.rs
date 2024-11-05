use terrazzo_pty::ResizeTerminalError;
use tracing::debug;
use tracing::error;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub async fn resize(
    terminal_id: &TerminalId,
    rows: i32,
    cols: i32,
) -> Result<(), ResizeOperationError> {
    debug!(rows, cols, "Size");
    let processes = get_processes();
    let entry = {
        let Some(entry) = processes.get(terminal_id) else {
            return Err(ResizeOperationError::TerminalNotFound {
                terminal_id: terminal_id.clone(),
            });
        };
        entry.value().1.clone()
    };
    let input = entry.input().await;
    let () = input.resize(rows as u16, cols as u16).await?;
    debug!("Done");
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ResizeOperationError {
    #[error("ResizeTerminalError: {0}")]
    ResizeTerminalError(#[from] ResizeTerminalError),

    #[error("TerminalNotFound: {terminal_id}")]
    TerminalNotFound { terminal_id: TerminalId },
}
