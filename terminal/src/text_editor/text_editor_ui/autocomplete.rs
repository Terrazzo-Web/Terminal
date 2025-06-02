use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::debounce::DoDebounce;
use wasm_bindgen_futures::spawn_local;
use web_sys::FocusEvent;
use web_sys::KeyboardEvent;
use web_sys::MouseEvent;

use super::path_selector::SafeHtmlInputElement;
use crate::frontend::menu::before_menu;
use crate::text_editor::base_path_autocomplete;

#[autoclone]
#[html]
#[template(tag = ul)]
pub fn show_autocomplete(
    input: Arc<OnceLock<SafeHtmlInputElement>>,
    autocomplete_sig: XSignal<Option<Vec<String>>>,
    #[signal] autocomplete: Option<Vec<String>>,
) -> XElement {
    let Some(autocomplete) = autocomplete else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    let items = autocomplete.into_iter().map(|item| {
        li(
            "{item}",
            mousedown = move |ev: MouseEvent| {
                autoclone!(input, autocomplete_sig);
                ev.prevent_default();
                ev.stop_propagation();
                let input_element = input.get().or_throw("Input element not set");
                input_element.set_value(&item);
                do_autocomplete_impl(input.clone(), autocomplete_sig.clone());
            },
        )
    });
    tag(class = super::style::autocomplete, items..)
}

#[autoclone]
pub fn start_autocomplete(
    input: Arc<OnceLock<SafeHtmlInputElement>>,
    autocomplete: XSignal<Option<Vec<String>>>,
) -> impl Fn(FocusEvent) {
    move |_| {
        *before_menu() = Some(Box::new(move || {
            autoclone!(input);
            let input_element = input.get().or_throw("Input element not set");
            input_element.blur().or_throw("Can't blur() input element")
        }));
        autocomplete.set(Some(Default::default()));
    }
}

pub fn stop_autocomplete(autocomplete: XSignal<Option<Vec<String>>>) -> impl Fn(FocusEvent) {
    move |_| {
        autocomplete.set(None);
    }
}

pub fn do_autocomplete(
    input: Arc<OnceLock<SafeHtmlInputElement>>,
    autocomplete: XSignal<Option<Vec<String>>>,
) -> impl Fn(KeyboardEvent) {
    Duration::from_millis(250)
        .debounce(move |_| do_autocomplete_impl(input.clone(), autocomplete.clone()))
}

#[autoclone]
fn do_autocomplete_impl(
    input: Arc<OnceLock<SafeHtmlInputElement>>,
    autocomplete: XSignal<Option<Vec<String>>>,
) {
    let input_element = input.get().or_throw("Input element not set");
    let value = input_element.value();
    spawn_local(async move {
        autoclone!(autocomplete);
        let autocompletes = base_path_autocomplete(value)
            .await
            .or_else_throw(|error| format!("Autocomplete failed: {error}"));
        autocomplete.update(|old| {
            if old.is_some() {
                Some(Some(autocompletes))
            } else {
                None
            }
        });
    });
}
