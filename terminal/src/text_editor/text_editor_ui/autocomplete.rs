use std::sync::Arc;
use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use web_sys::FocusEvent;

use super::path_selector::SafeHtmlInputElement;
use crate::frontend::menu::before_menu;

#[html]
#[template(tag = ul)]
pub fn show_autocomplete(#[signal] autocomplete: Option<Vec<String>>) -> XElement {
    let Some(autocomplete) = autocomplete else {
        return tag(style::visibility = "hidden", style::display = "none");
    };
    let items = autocomplete.into_iter().map(|item| li("{item}"));
    tag(class = super::style::autocomplete, items..)
}

#[autoclone]
pub fn start_autocomplete(input: Arc<OnceLock<SafeHtmlInputElement>>) -> impl Fn(FocusEvent) {
    move |_| {
        *before_menu() = Some(Box::new(move || {
            autoclone!(input);
            let input = input.get().or_throw("Input element not set");
            input.blur().or_throw("Can't blur() input element")
        }));
    }
}

pub fn stop_autocomplete(autocomplete: XSignal<Option<Vec<String>>>) -> impl Fn(FocusEvent) {
    move |_| {
        autocomplete.set(None);
    }
}
