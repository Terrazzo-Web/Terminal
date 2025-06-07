#![cfg(feature = "client")]

use server_fn::ServerFnError;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use self::editor::editor;
use self::path_selector::base_path_selector;
use self::path_selector::file_path_selector;
use super::load_file;
use super::state;
use crate::frontend::menu::menu;

mod autocomplete;
mod code_mirror;
mod editor;
mod path_selector;

stylance::import_crate_style!(style, "src/text_editor/text_editor.scss");

#[autoclone]
#[html]
#[template]
pub fn text_editor() -> XElement {
    let base_path = XSignal::new("base-path", String::default());
    let file_path = XSignal::new("file-path", String::default());
    let content = XSignal::new("content", None);

    spawn_local(async move {
        autoclone!(base_path, file_path);
        if let Ok(p) = state::base_path::get().await {
            base_path.set(p);
        }
        if let Ok(p) = state::file_path::get().await {
            file_path.set(p);
        }
    });

    let base_path_subscriber = make_subs("base_path", base_path.clone(), state::base_path::set);
    let file_path_subscriber = make_subs("file_path", file_path.clone(), state::file_path::set);

    let file_async_view = file_path.add_subscriber(move |file_path| {
        autoclone!(base_path, content);
        content.set(None);
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
                before_render = move |_| {
                    let _moved = &file_async_view;
                },
                menu(),
                base_path_selector(base_path.clone()),
                file_path_selector(base_path, file_path),
            ),
            editor(content),
            after_render = move |_| {
                let _ = &base_path_subscriber;
                let _ = &file_path_subscriber;
            },
        ),
    )
}

fn make_subs(
    name: &'static str,
    path: XSignal<String>,
    setter: impl AsyncFn(String) -> Result<(), ServerFnError> + Copy + 'static,
) -> Consumers {
    path.add_subscriber(move |p| {
        spawn_local(async move {
            let () = setter(p)
                .await
                .unwrap_or_else(|error| warn!("Failed to set {name}: {error}"));
        })
    })
}
