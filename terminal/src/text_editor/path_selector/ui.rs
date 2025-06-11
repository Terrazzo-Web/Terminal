#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;

use nameth::NamedEnumValues;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast as _;
use web_sys::HtmlInputElement;

use super::PathSelector;
use crate::assets::icons;
use crate::text_editor::autocomplete::ui::do_autocomplete;
use crate::text_editor::autocomplete::ui::show_autocomplete;
use crate::text_editor::autocomplete::ui::start_autocomplete;
use crate::text_editor::autocomplete::ui::stop_autocomplete;
use crate::text_editor::style;
use crate::text_editor::ui::TextEditor;

impl TextEditor {
    pub fn base_path_selector(&self) -> XElement {
        path_selector_impll(PathSelector::BasePath, None, self.base_path.clone())
    }

    pub fn file_path_selector(&self) -> XElement {
        path_selector_impll(
            PathSelector::FilePath,
            Some(self.base_path.clone()),
            self.file_path.clone(),
        )
    }
}

#[autoclone]
#[html]
#[template(tag = div)]
fn path_selector_impll(
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    path: XSignal<Arc<str>>,
) -> XElement {
    let autocomplete: XSignal<Option<Vec<String>>> = XSignal::new(kind.name(), None);
    let input: Arc<OnceLock<UiThreadSafe<HtmlInputElement>>> = OnceLock::new().into();
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
        class = style::path_selector,
        img(class = style::path_selector_icon, src = kind.icon()),
        div(
            class = style::path_selector_widget,
            input(
                before_render = move |element| {
                    autoclone!(input);
                    let _ = &onchange;
                    let element = element
                        .dyn_into::<HtmlInputElement>()
                        .or_throw("Not an HtmlInputElement");
                    input
                        .set(element.into())
                        .or_throw("Input element already set");
                },
                r#type = "text",
                class = style::path_selector_field,
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

impl PathSelector {
    pub fn icon(self) -> icons::Icon {
        match self {
            Self::BasePath => icons::slash(),
            Self::FilePath => icons::chevron_double_right(),
        }
    }
}
