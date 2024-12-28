use named::named;
use serde::Deserialize;
use serde::Serialize;

use crate::terminal_id::TerminalId;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

const ERROR_HEADER: &str = "terrazzo-error";
const CORRELATION_ID: &str = "terrazzo-correlation-id";

const NEWLINE: u8 = b'\n';

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    pub rows: i32,
    pub cols: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    terminal_id: TerminalId,
    data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalDef {
    pub id: TerminalId,
    pub title: String,
}

#[named]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RegisterTerminalQuery {
    pub mode: RegisterTerminalMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RegisterTerminalMode {
    Create,
    Reopen,
}
