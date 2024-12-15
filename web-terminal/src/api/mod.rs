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
pub struct TerminalDef<T = String> {
    pub id: TerminalId,
    pub title: T,
    pub order: i32,
}

#[named]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterTerminalRequest {
    pub mode: RegisterTerminalMode,
    pub def: TerminalDef,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RegisterTerminalMode {
    Create,
    Reopen,
}

#[allow(unused)]
pub static APPLICATION_JSON: &str = "application/json";

#[test]
#[cfg(all(test, feature = "server"))]
fn application_json_test() {
    assert_eq!(APPLICATION_JSON, terrazzo::mime::APPLICATION_JSON);
}
