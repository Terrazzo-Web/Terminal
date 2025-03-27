use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tracing::trace;
use trz_gateway_common::http_error::IsHttpError;

use super::get_processes;
use crate::api::TabTitle;
use crate::terminal_id::TerminalId;

pub fn set_title(
    terminal_id: &TerminalId,
    new_title: TabTitle<String>,
) -> Result<(), SetTitleError> {
    trace!("Setting title of {terminal_id} to {new_title:?}");
    let processes = get_processes();
    let Some(mut entry) = processes.get_mut(terminal_id) else {
        return Err(SetTitleError::TerminalNotFound {
            terminal_id: terminal_id.to_owned(),
        });
    };
    entry.0.title = new_title;
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] Terminal not found {terminal_id}", n = self.name())]
    TerminalNotFound { terminal_id: TerminalId },
}

impl IsHttpError for SetTitleError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TerminalNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}
