#![cfg(feature = "client")]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use self::path_selector::base_path_selector;
use self::path_selector::path_selector;
use crate::frontend::menu::menu;

mod autocomplete;
mod path_selector;

stylance::import_crate_style!(style, "src/text_editor/text_editor.scss");

#[html]
#[template]
pub fn text_editor() -> XElement {
    div(
        style = "height: 100%;",
        div(
            key = "text-editor",
            class = style::text_editor,
            div(
                class = style::header,
                menu(),
                base_path_selector(),
                path_selector(),
            ),
            div(class = style::body, "hello"),
        ),
    )
}
