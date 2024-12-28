#![cfg(feature = "server")]

use std::sync::Arc;
use std::sync::OnceLock;

use dashmap::DashMap;
use terrazzo_pty::lease::ProcessIoEntry;

use super::terminal_id::TerminalId;
use crate::api::TerminalDef;

pub mod close;
pub mod list;
pub mod resize;
pub mod set_title;
pub mod stream;
pub mod write;

fn get_processes() -> &'static dashmap::DashMap<TerminalId, (TerminalDef, Arc<ProcessIoEntry>)> {
    static PROCESSES: OnceLock<dashmap::DashMap<TerminalId, (TerminalDef, Arc<ProcessIoEntry>)>> =
        OnceLock::new();
    PROCESSES.get_or_init(DashMap::new)
}
