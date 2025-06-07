use std::ops::Deref;
use std::sync::Arc;
use std::sync::OnceLock;

use nameth::NamedEnumValues;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast as _;
use web_sys::HtmlInputElement;

use super::autocomplete::*;
use crate::text_editor::PathSelector;

pub fn base_path_selector(base_path: XSignal<String>) -> XElement {
    path_selector_impll(PathSelector::BasePath, None, base_path)
}

pub fn file_path_selector(base_path: XSignal<String>, file_path: XSignal<String>) -> XElement {
    path_selector_impll(PathSelector::FilePath, Some(base_path), file_path)
}

#[autoclone]
#[html]
#[template(tag = div)]
fn path_selector_impll(
    kind: PathSelector,
    prefix: Option<XSignal<String>>,
    path: XSignal<String>,
) -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new(kind.name(), None);
    let input: Arc<OnceLock<SafeHtmlInputElement>> = OnceLock::new().into();
    let do_autocomplete = Ptr::new(do_autocomplete(
        input.clone(),
        autocomplete.clone(),
        kind,
        prefix.clone(),
    ));
    let onchange = path.add_subscriber(move |new| {
        autoclone!(input);
        if let Some(input) = input.get() {
            input.set_value(&new);
        }
    });
    tag(
        class = super::style::path_selector,
        img(class = super::style::icon, src = kind.icon()),
        div(
            class = super::style::selector,
            input(
                before_render = move |element| {
                    autoclone!(input);
                    let _ = &onchange;
                    let element = element
                        .dyn_into::<HtmlInputElement>()
                        .or_throw("Not an HtmlInputElement");
                    input
                        .set(SafeHtmlInputElement(element.into()))
                        .or_throw("Input element already set");
                },
                r#type = "text",
                class = super::style::selector,
                focus =
                    start_autocomplete(kind, prefix.clone(), input.clone(), autocomplete.clone()),
                blur = stop_autocomplete(path.clone(), input.clone(), autocomplete.clone()),
                keydown = move |_| {
                    autoclone!(do_autocomplete);
                    do_autocomplete(())
                },
                click = move |_| {
                    autoclone!(do_autocomplete);
                    do_autocomplete(())
                },
            ),
            show_autocomplete(
                kind,
                prefix.clone(),
                input,
                autocomplete.clone(),
                autocomplete,
                path,
            ),
        ),
    )
}

pub struct SafeHtmlInputElement(HtmlInputElement);
unsafe impl Send for SafeHtmlInputElement {}
unsafe impl Sync for SafeHtmlInputElement {}

impl Deref for SafeHtmlInputElement {
    type Target = HtmlInputElement;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
