use serde::Deserialize;
use serde::Serialize;

use crate::api::client_address::ClientAddress;
use crate::terminal_id::TerminalId;

pub const STREAMING_WINDOW_SIZE: usize = 200 * 1000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "r"))]
    pub rows: i32,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub cols: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub terminal_id: TerminalId,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    pub data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalAddress {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub id: TerminalId,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "a"))]
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
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "a"))]
    pub address: TerminalAddress,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub title: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "o"))]
    pub order: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabTitle<T> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub shell_title: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "o"))]
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
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "m"))]
    pub mode: RegisterTerminalMode,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    pub def: TerminalDef,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterTerminalMode {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "C"))]
    Create,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "R"))]
    Reopen,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteRequest<T = TerminalAddress> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub terminal: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    pub data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResizeRequest<T = TerminalAddress> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub terminal: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "s"))]
    pub size: Size,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub force: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetTitleRequest<T = TerminalAddress> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub terminal: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "v"))]
    pub title: TabTitle<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AckRequest<T = TerminalAddress> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub terminal: T,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    pub ack: usize,
}
