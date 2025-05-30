use serde::Deserialize;
use serde::Serialize;

use self::client_address::ClientAddress;
use crate::terminal_id::TerminalId;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
use trz_gateway_common::id::ClientName;

#[cfg(all(feature = "client", not(feature = "server")))]
use self::client_name::ClientName;

pub mod client_address;
pub mod client_name;

const CORRELATION_ID: &str = "terrazzo-correlation-id";
const KEEPALIVE_TTL_HEADER: &str = "terrazzo-keepalive-ttl";

const NEWLINE: u8 = b'\n';

pub const STREAMING_WINDOW_SIZE: usize = 200 * 1000;

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
pub struct TerminalAddress {
    pub id: TerminalId,
    pub via: ClientAddress,
}

mod display_terminal_address {
    use std::fmt::Display;

    impl Display for super::TerminalAddress {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} via {}", self.id, self.via)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalDefImpl<T> {
    pub address: TerminalAddress,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterTerminalRequest {
    pub mode: RegisterTerminalMode,
    pub def: TerminalDef,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteRequest<T = TerminalAddress> {
    terminal: T,
    data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResizeRequest<T = TerminalAddress> {
    terminal: T,
    size: Size,
    force: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetTitleRequest<T = TerminalAddress> {
    terminal: T,
    title: TabTitle<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AckRequest<T = TerminalAddress> {
    terminal: T,
    ack: usize,
}
