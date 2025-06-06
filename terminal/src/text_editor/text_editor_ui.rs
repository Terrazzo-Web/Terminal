#![cfg(feature = "client")]

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen_futures::spawn_local;

use self::path_selector::base_path_selector;
use self::path_selector::file_path_selector;
use crate::frontend::menu::menu;
use crate::text_editor::load_file;
use crate::text_editor::text_editor_ui::editor::editor;

mod autocomplete;
mod code_mirror;
mod editor;
mod path_selector;

stylance::import_crate_style!(style, "src/text_editor/text_editor_ui.scss");

#[autoclone]
#[html]
#[template]
pub fn text_editor() -> XElement {
    let base_path = XSignal::new("base-path", String::default());
    let file_path = XSignal::new("file-path", String::default());
    let content = XSignal::new("content", None);

    let file_async_view = file_path.view("content", move |file_path| {
        autoclone!(base_path, content);
        let task = async move {
            autoclone!(base_path, content, file_path);
            let data = load_file(base_path.get_value_untracked(), file_path)
                .await
                .unwrap_or_else(|error| Some(error.to_string().into()));
            content.set(data);
        };
        spawn_local(task);
    });
    div(
        style = "height: 100%;",
        div(
            key = "text-editor",
            class = style::text_editor,
            div(
                class = style::header,
                before_render = move |_| file_async_view.get_value_untracked(),
                menu(),
                base_path_selector(base_path.clone()),
                file_path_selector(base_path, file_path),
            ),
            editor(content),
        ),
    )
}
