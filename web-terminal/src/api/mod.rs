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
pub struct TerminalDefImpl<T> {
    pub id: TerminalId,
    pub title: T,
    pub order: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabTitle<T> {
    pub shell_title: T,
    pub override_title: Option<T>,
}

#[cfg(feature = "client")]
impl<T> TabTitle<T> {
    pub fn map<U>(self, f: impl Fn(T) -> U) -> TabTitle<U> {
        TabTitle {
            shell_title: f(self.shell_title),
            override_title: self.override_title.map(f),
        }
    }
}

pub type TerminalDef = TerminalDefImpl<TabTitle<String>>;

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
