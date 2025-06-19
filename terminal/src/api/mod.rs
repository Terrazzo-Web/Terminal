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
    #[cfg_attr(not(debug_assertions), serde(rename = "r"))]
    pub rows: i32,
    #[cfg_attr(not(debug_assertions), serde(rename = "c"))]
    pub cols: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    terminal_id: TerminalId,
    #[cfg_attr(not(debug_assertions), serde(rename = "d"))]
    data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalAddress {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    pub id: TerminalId,
    #[cfg_attr(not(debug_assertions), serde(rename = "a"))]
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
    #[cfg_attr(not(debug_assertions), serde(rename = "a"))]
    pub address: TerminalAddress,
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    pub title: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "o"))]
    pub order: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabTitle<T> {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    pub shell_title: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "o"))]
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
    #[cfg_attr(not(debug_assertions), serde(rename = "m"))]
    pub mode: RegisterTerminalMode,
    #[cfg_attr(not(debug_assertions), serde(rename = "d"))]
    pub def: TerminalDef,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterTerminalMode {
    #[cfg_attr(not(debug_assertions), serde(rename = "C"))]
    Create,
    #[cfg_attr(not(debug_assertions), serde(rename = "R"))]
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
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    terminal: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "d"))]
    data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResizeRequest<T = TerminalAddress> {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    terminal: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "s"))]
    size: Size,
    #[cfg_attr(not(debug_assertions), serde(rename = "f"))]
    force: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetTitleRequest<T = TerminalAddress> {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    terminal: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "v"))]
    title: TabTitle<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AckRequest<T = TerminalAddress> {
    #[cfg_attr(not(debug_assertions), serde(rename = "t"))]
    terminal: T,
    #[cfg_attr(not(debug_assertions), serde(rename = "d"))]
    ack: usize,
}
