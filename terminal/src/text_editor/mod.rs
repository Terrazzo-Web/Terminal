#![cfg(feature = "client")]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use web_sys::FocusEvent;

use crate::assets::icons;
use crate::frontend::menu::menu;

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

#[html]
#[template(tag = div)]
fn base_path_selector() -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new("base-path-selector", None);
    tag(
        class = style::path_selector,
        img(class = style::icon, src = icons::slash()),
        div(
            class = style::selector,
            input(
                r#type = "text",
                class = style::selector,
                focus = start_autocomplete("/".to_owned(), autocomplete.clone()),
                blur = stop_autocomplete(autocomplete.clone()),
            ),
            show_autocomplete(autocomplete),
        ),
    )
}

#[html]
#[template(tag = div)]
fn path_selector() -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new("path-selector", None);
    tag(
        class = style::path_selector,
        img(class = style::icon, src = icons::chevron_double_right()),
        input(
            r#type = "text",
            class = style::selector,
            focus = start_autocomplete("/".to_owned(), autocomplete.clone()),
            blur = stop_autocomplete(autocomplete.clone()),
        ),
        show_autocomplete(autocomplete),
    )
}

#[html]
#[template(tag = ul)]
fn show_autocomplete(#[signal] autocomplete: Option<Vec<String>>) -> XElement {
    let Some(autocomplete) = autocomplete else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    let items = autocomplete.into_iter().map(|item| li("{item}"));
    tag(class = style::autocomplete, items..)
}

fn start_autocomplete(
    base: String,
    autocomplete: XSignal<Option<Vec<String>>>,
) -> impl Fn(FocusEvent) {
    move |_| {
        autocomplete.set(vec![
            base.to_owned(),
            "a1".to_owned(),
            "a2".to_owned(),
            "a3".to_owned(),
            "a4".to_owned(),
        ]);
    }
}

fn stop_autocomplete(autocomplete: XSignal<Option<Vec<String>>>) -> impl Fn(FocusEvent) {
    move |_| {
        autocomplete.set(None);
    }
}
