use super::get_processes;
use crate::terminal_id::TerminalId;

pub fn close(terminal_id: &TerminalId) -> Result<(), CloseProcessError> {
    get_processes()
        .remove(terminal_id)
        .map(|_deleted_entry| ())
        .ok_or_else(move || CloseProcessError::NotFound {
            terminal_id: terminal_id.to_owned(),
        })
}

#[derive(thiserror::Error, Debug)]
pub enum CloseProcessError {
    #[error("NotFound: {terminal_id}")]
    NotFound { terminal_id: TerminalId },
}
