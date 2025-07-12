#![cfg(feature = "terminal")]

use terrazzo::prelude::XSignal;
use terrazzo::prelude::XString;

use crate::api::shared::terminal_schema::TabTitle;
use crate::api::shared::terminal_schema::TerminalDefImpl;

pub mod list;
pub mod new_id;
pub mod resize;
pub mod set_order;
pub mod set_title;
pub mod stream;
pub mod write;

pub type LiveTerminalDef = TerminalDefImpl<XSignal<TabTitle<XString>>>;

pub const BASE_TERMINAL_URL: &str = "/api/terminal";
