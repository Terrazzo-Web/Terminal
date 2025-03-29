use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use trz_gateway_common::http_error::IsHttpError;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub fn close(terminal_id: &TerminalId) -> Result<(), CloseProcessError> {
    get_processes()
        .remove(terminal_id)
        .map(|_deleted_entry| ())
        .ok_or_else(move || CloseProcessError::TerminalNotFound {
            terminal_id: terminal_id.to_owned(),
        })
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum CloseProcessError {
    #[error("[{n}] Terminal not found {terminal_id}", n = self.name())]
    TerminalNotFound { terminal_id: TerminalId },
}

impl IsHttpError for CloseProcessError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TerminalNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}
