use std::ops::Deref;
use std::sync::Arc;
use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

use super::autocomplete::*;
use crate::assets::icons;
use crate::text_editor::base_path_autocomplete;

pub fn base_path_selector() -> XElement {
    path_selector_impll("base-path-selector", icons::slash())
}

pub fn path_selector() -> XElement {
    path_selector_impll("path-selector", icons::chevron_double_right())
}

#[autoclone]
#[html]
#[template(tag = div)]
fn path_selector_impll(name: &'static str, icon_src: icons::Icon) -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new(name, None);
    let input: Arc<OnceLock<SafeHtmlInputElement>> = OnceLock::new().into();
    tag(
        class = super::style::path_selector,
        img(class = super::style::icon, src = icon_src),
        div(
            class = super::style::selector,
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
                class = super::style::selector,
                focus = start_autocomplete(input.clone()),
                blur = stop_autocomplete(autocomplete.clone()),
                keypress = move |_| {
                    autoclone!(input, autocomplete);
                    let value = input.get().unwrap().value();
                    spawn_local(async move {
                        autoclone!(autocomplete);
                        let autocompletes = base_path_autocomplete(value)
                            .await
                            .or_else_throw(|error| format!("Autocomplete failed: {error}"));
                        autocomplete.set(autocompletes);
                    });
                },
            ),
            show_autocomplete(autocomplete),
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
