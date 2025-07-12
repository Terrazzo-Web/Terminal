#![cfg(feature = "server")]

use std::sync::Arc;
use std::sync::OnceLock;

use dashmap::DashMap;
use terrazzo_pty::lease::ProcessIoEntry;

use super::terminal_id::TerminalId;
use crate::api::shared::terminal_schema::TerminalDef;

pub mod close;
pub mod io;
pub mod list;
pub mod resize;
pub mod set_title;
pub mod stream;
pub mod write;

pub fn get_processes() -> &'static DashMap<TerminalId, (TerminalDef, Arc<ProcessIoEntry>)> {
    static PROCESSES: OnceLock<DashMap<TerminalId, (TerminalDef, Arc<ProcessIoEntry>)>> =
        OnceLock::new();
    PROCESSES.get_or_init(DashMap::new)
}

pub fn next_terminal_id() -> i32 {
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering::SeqCst;
    static NEXT: AtomicI32 = AtomicI32::new(1);
    NEXT.fetch_add(1, SeqCst)
}
