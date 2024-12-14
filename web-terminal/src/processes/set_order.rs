use tracing::trace;
use tracing::warn;

use super::get_processes;
use crate::terminal_id::TerminalId;

pub fn set_order(ids: Vec<TerminalId>) {
    trace!("Setting tab order: {ids:?}");
    let processes = get_processes();
    for (order, terminal_id) in ids.into_iter().enumerate() {
        let Some(mut entry) = processes.get_mut(&terminal_id) else {
            warn!("Terminal '{terminal_id}' not found");
            continue;
        };
        entry.0.order = order as i32;
    }
}
