#![cfg(feature = "client")]

use std::sync::Arc;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce;
use wasm_bindgen_futures::spawn_local;
use web_sys::FocusEvent;
use web_sys::HtmlInputElement;
use web_sys::MouseEvent;

use self::diagnostics::Instrument as _;
use self::diagnostics::info;
use super::server_fn::AutocompleteItem;
use super::server_fn::autocomplete_path;
use crate::frontend::element_capture::ElementCapture;
use crate::frontend::menu::before_menu;
use crate::text_editor::manager::TextEditorManager;
use crate::text_editor::path_selector::schema::PathSelector;
use crate::text_editor::style;

#[autoclone]
#[html]
#[template(tag = ul)]
pub fn show_autocomplete(
    manager: Ptr<TextEditorManager>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    input: ElementCapture<HtmlInputElement>,
    autocomplete_sig: XSignal<Option<Vec<AutocompleteItem>>>,
    #[signal] autocomplete: Option<Vec<AutocompleteItem>>,
    path: XSignal<Arc<str>>,
) -> XElement {
    let Some(autocomplete) = autocomplete else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    let items = autocomplete.into_iter().map(|item| {
        let item_display = if item.is_dir && item.path != "/" {
            format!("{}/", item.path)
        } else if item.path.trim().is_empty() {
            "\u{00A0}".into()
        } else {
            item.path.to_owned()
        };
        li(
            "{item_display}",
            mousedown = move |ev: MouseEvent| {
                autoclone!(manager, input, autocomplete_sig, prefix, path);
                ev.prevent_default();
                ev.stop_propagation();
                {
                    input.with(|i| i.set_value(&item.path));
                    path.set(item.path.as_str());
                }
                do_autocomplete_impl(
                    manager.clone(),
                    kind,
                    prefix.clone(),
                    input.clone(),
                    autocomplete_sig.clone(),
                );
            },
        )
    });
    tag(class = style::path_selector_autocomplete, items..)
}

pub fn start_autocomplete(
    manager: Ptr<TextEditorManager>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    input: ElementCapture<HtmlInputElement>,
    autocomplete: XSignal<Option<Vec<AutocompleteItem>>>,
) -> impl Fn(FocusEvent) {
    move |_| {
        let input_element_blur = scopeguard::guard(input.clone(), |input| {
            input
                .with(|i| i.blur())
                .or_throw("Can't blur() input element")
        });
        *before_menu() = Some(Box::new(move || drop(input_element_blur)));
        autocomplete.set(Some(Default::default()));
        do_autocomplete_impl(
            manager.clone(),
            kind,
            prefix.clone(),
            input.clone(),
            autocomplete.clone(),
        );
    }
}

pub fn stop_autocomplete(
    path: XSignal<Arc<str>>,
    input: ElementCapture<HtmlInputElement>,
    autocomplete: XSignal<Option<Vec<AutocompleteItem>>>,
) -> impl Fn(FocusEvent) {
    move |_| {
        let value = input.with(|i| i.value());
        info!("Update path to {value}");
        path.set(value);
        autocomplete.set(None);
    }
}

pub fn do_autocomplete(
    manager: Ptr<TextEditorManager>,
    input: ElementCapture<HtmlInputElement>,
    autocomplete: XSignal<Option<Vec<AutocompleteItem>>>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
) -> impl Fn(()) {
    Duration::from_millis(250).debounce(move |()| {
        do_autocomplete_impl(
            manager.clone(),
            kind,
            prefix.clone(),
            input.clone(),
            autocomplete.clone(),
        )
    })
}

#[autoclone]
fn do_autocomplete_impl(
    manager: Ptr<TextEditorManager>,
    kind: PathSelector,
    prefix: Option<XSignal<Arc<str>>>,
    input: ElementCapture<HtmlInputElement>,
    autocomplete: XSignal<Option<Vec<AutocompleteItem>>>,
) {
    let value = input.with(|i| i.value());
    let do_autocomplete_async = async move {
        autoclone!(autocomplete);
        let autocompletes = autocomplete_path(
            manager.remote.clone(),
            kind,
            prefix
                .as_ref()
                .map(XSignal::get_value_untracked)
                .unwrap_or_default(),
            value,
        )
        .await
        .or_else_throw(|error| format!("Autocomplete failed: {error}"));
        autocomplete.update(|old| {
            if old.is_some() {
                Some(Some(autocompletes))
            } else {
                None
            }
        });
    };
    spawn_local(do_autocomplete_async.in_current_span());
}
