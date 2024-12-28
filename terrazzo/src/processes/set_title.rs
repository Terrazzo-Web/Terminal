use tracing::trace;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub fn set_title(terminal_id: &TerminalId, new_title: String) -> std::io::Result<()> {
    trace!("Setting title of {terminal_id} to {new_title:?}");
    let processes = get_processes();
    let Some(mut entry) = processes.get_mut(terminal_id) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Terminal '{terminal_id}' not found"),
        ));
    };
    entry.0.title = new_title;
    Ok(())
}
