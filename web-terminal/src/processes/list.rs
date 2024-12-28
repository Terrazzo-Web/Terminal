use super::get_processes;
use crate::api::TerminalDef;

pub fn list() -> Vec<TerminalDef> {
    let mut processes = get_processes()
        .iter()
        .map(|entry| entry.value().0.clone())
        .collect::<Vec<_>>();
    processes.sort_by_key(|t| t.order);
    return processes;
}
