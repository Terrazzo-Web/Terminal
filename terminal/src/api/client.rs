use terrazzo::prelude::XSignal;
use terrazzo::prelude::XString;

use super::TabTitle;
use super::TerminalDefImpl;

pub mod new_id;
pub mod remotes;
mod request;
pub mod resize;
pub mod set_order;
pub mod set_title;
pub mod stream;
pub mod terminals;
pub mod write;

pub type LiveTerminalDef = TerminalDefImpl<XSignal<TabTitle<XString>>>;
