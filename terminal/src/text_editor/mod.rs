#![cfg(feature = "client")]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

#[html]
#[template]
pub fn text_editor() -> XElement {
    div("Text editor")
}
