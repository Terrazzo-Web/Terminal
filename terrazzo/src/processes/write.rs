use tokio::io::AsyncWriteExt as _;
use tracing::trace;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub async fn write(terminal_id: &TerminalId, data: &[u8]) -> std::io::Result<()> {
    trace!("Writing {}", String::from_utf8_lossy(data).escape_default());
    let processes = get_processes();
    let entry = {
        let Some(entry) = processes.get(terminal_id) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Terminal '{terminal_id}' not found"),
            ));
        };
        entry.value().1.clone()
    };
    let mut input = entry.input().await;
    return input.write_all(data).await;
}
