use std::time::Duration;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use terrazzo_pty::ProcessInput;
use terrazzo_pty::pty::PtyError;
use terrazzo_pty::size::Size;
use tracing::debug;
use tracing::error;
use trz_gateway_common::http_error::IsHttpError;

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
        let () = input
            .resize(Size::new(rows as u16 - 1, cols as u16 - 1))
            .map_err(ResizeOperationError::Resize)?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let () = input
        .resize(Size::new(rows as u16, cols as u16))
        .map_err(ResizeOperationError::Resize)?;
    debug!("Done");
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ResizeOperationError {
    #[error("[{n}] Failed to resize: {0}", n = self.name())]
    Resize(PtyError),

    #[error("[{n}] Terminal not found {terminal_id}", n = self.name())]
    TerminalNotFound { terminal_id: TerminalId },
}

impl IsHttpError for ResizeOperationError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Resize { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TerminalNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}
