use std::time::Duration;

use terrazzo_pty::ProcessInput;
use terrazzo_pty::pty::PtyError;
use terrazzo_pty::size::Size;
use tracing::debug;
use tracing::error;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub async fn resize(
    terminal_id: &TerminalId,
    rows: i32,
    cols: i32,
    force: bool,
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
    let ProcessInput(input) = &*input;
    if force {
        debug!("Forcing resize");
        let () = input.resize(Size::new(rows as u16 - 1, cols as u16 - 1))?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let () = input.resize(Size::new(rows as u16, cols as u16))?;
    debug!("Done");
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ResizeOperationError {
    #[error("ResizeTerminalError: {0}")]
    PtyError(#[from] PtyError),

    #[error("TerminalNotFound: {terminal_id}")]
    TerminalNotFound { terminal_id: TerminalId },
}
