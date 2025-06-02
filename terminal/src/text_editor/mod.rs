#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast;
use web_sys::FocusEvent;
use web_sys::HtmlInputElement;

use crate::assets::icons;
use crate::frontend::menu::before_menu;
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

fn base_path_selector() -> XElement {
    path_selector_impll("base-path-selector", icons::slash())
}

fn path_selector() -> XElement {
    path_selector_impll("path-selector", icons::chevron_double_right())
}

#[autoclone]
#[html]
#[template(tag = div)]
fn path_selector_impll(name: &'static str, icon_src: icons::Icon) -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new(name, None);
    let input: Arc<OnceLock<SafeHtmlInputElement>> = OnceLock::new().into();
    tag(
        class = style::path_selector,
        img(class = style::icon, src = icon_src),
        div(
            class = style::selector,
            input(
                before_render = move |element| {
                    autoclone!(input);
                    let element = element
                        .dyn_into::<HtmlInputElement>()
                        .or_throw("Not an HtmlInputElement");
                    input
                        .set(SafeHtmlInputElement(element))
                        .or_throw("Input element already set");
                },
                r#type = "text",
                class = style::selector,
                focus = start_autocomplete("/".to_owned(), autocomplete.clone(), input.clone()),
                blur = stop_autocomplete(autocomplete.clone()),
            ),
            show_autocomplete(autocomplete),
        ),
    )
}

struct SafeHtmlInputElement(HtmlInputElement);
unsafe impl Send for SafeHtmlInputElement {}
unsafe impl Sync for SafeHtmlInputElement {}

#[html]
#[template(tag = ul)]
fn show_autocomplete(#[signal] autocomplete: Option<Vec<String>>) -> XElement {
    let Some(autocomplete) = autocomplete else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    let items = autocomplete.into_iter().map(|item| li("{item}"));
    tag(class = style::autocomplete, items..)
}

#[autoclone]
fn start_autocomplete(
    base: String,
    autocomplete: XSignal<Option<Vec<String>>>,
    input: Arc<OnceLock<SafeHtmlInputElement>>,
) -> impl Fn(FocusEvent) {
    move |_| {
        *before_menu() = Some(Box::new(move || {
            autoclone!(input);
            let input = input.get().or_throw("Input element not set");
            input.0.blur().or_throw("Can't blur() input element")
        }));
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
