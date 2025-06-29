#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::OnceLock;

use nameth::NamedEnumValues as _;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast as _;
use web_sys::HtmlInputElement;

use super::PathSelector;
use crate::assets::icons;
use crate::text_editor::autocomplete::AutocompleteItem;
use crate::text_editor::autocomplete::ui::do_autocomplete;
use crate::text_editor::autocomplete::ui::show_autocomplete;
use crate::text_editor::autocomplete::ui::start_autocomplete;
use crate::text_editor::autocomplete::ui::stop_autocomplete;
use crate::text_editor::manager::TextEditor;
use crate::text_editor::style;

impl TextEditor {
    pub fn base_path_selector(self: &Arc<Self>) -> XElement {
        path_selector_impll(
            self.clone(),
            PathSelector::BasePath,
            None,
            self.base_path.clone(),
            self.force_edit_path.clone(),
        )
    }

    pub fn file_path_selector(self: &Arc<Self>) -> XElement {
        path_selector_impll(
            self.clone(),
            PathSelector::FilePath,
            Some(self.base_path.clone()),
            self.file_path.clone(),
            XSignal::new("unused-force-edit-path", false),
        )
    }
}

#[html]
#[template(tag = div)]
fn path_selector_impll(
    text_editor: Arc<TextEditor>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    path: XSignal<Arc<str>>,
    #[signal] mut force_edit_path: bool,
) -> XElement {
    let show_input = kind == PathSelector::FilePath || force_edit_path;
    tag(
        class = style::path_selector,
        style = (!show_input).then_some("width: auto;"),
        img(class = style::path_selector_icon, src = kind.icon()),
        if show_input {
            path_selector_input(text_editor, kind, prefix, path)
        } else {
            path_selector_display(path, force_edit_path_mut)
        },
    )
}

#[autoclone]
#[html]
fn path_selector_input(
    text_editor: Arc<TextEditor>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    path: XSignal<Arc<str>>,
) -> XElement {
    let autocomplete: XSignal<Option<Vec<AutocompleteItem>>> = XSignal::new(kind.name(), None);
    let input: Arc<OnceLock<UiThreadSafe<HtmlInputElement>>> = OnceLock::new().into();
    let do_autocomplete = Ptr::new(do_autocomplete(
        text_editor.clone(),
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
    div(
        class = style::path_selector_widget,
        key = "input",
        input(
            before_render = move |element| {
                autoclone!(input, path);
                let _ = &onchange;
                let element = element
                    .dyn_into::<HtmlInputElement>()
                    .or_throw("Not an HtmlInputElement");
                element.set_value(&path.get_value_untracked());
                input
                    .set(element.into())
                    .or_throw("Input element already set");
            },
            r#type = "text",
            class = style::path_selector_field,
            focus = start_autocomplete(
                text_editor.clone(),
                kind,
                prefix.clone(),
                input.clone(),
                autocomplete.clone(),
            ),
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
            text_editor,
            kind,
            prefix.clone(),
            input,
            autocomplete.clone(),
            autocomplete,
            path,
        ),
    )
}

#[html]
#[template(tag = div)]
fn path_selector_display(
    #[signal] path: Arc<str>,
    force_edit_path_mut: MutableSignal<bool>,
) -> XElement {
    div(
        class = style::path_selector_widget,
        key = "display",
        span(
            class = style::path_selector_field,
            dblclick = move |_ev| force_edit_path_mut.set(true),
            "{path}",
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
