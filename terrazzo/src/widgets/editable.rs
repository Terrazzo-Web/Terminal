use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use web_sys::Event;
use web_sys::FocusEvent;
use web_sys::HtmlInputElement;
use web_sys::MouseEvent;

use super::more_event::MoreEvent;

stylance::import_crate_style!(style, "src/widgets/editable.scss");

static EDITABLE_ELEMENT: &str = "Editable element";

#[html]
#[template]
pub fn editable<P, PI>(
    value: XSignal<XString>,
    editable: XSignal<bool>,
    #[signal] mut editing: bool,
    printed: impl FnOnce() -> PI + Clone + 'static,
) where
    XNode: From<P>,
    PI: IntoIterator<Item = P>,
{
    if editing {
        input!(move |t| show_editing(t, value.clone(), editing_mut.clone()))
    } else {
        span!(move |t| show_printed(t, editable.clone(), printed.clone(), editing_mut.clone()))
    }
}

#[html]
#[template]
fn show_editing(#[signal] mut content: XString, editing_mut: MutableSignal<bool>) {
    let editing_mut2 = editing_mut.clone();
    let content_mut2 = content_mut.clone();
    input!(
        r#type = "text",
        class = style::editing,
        click = move |ev: MouseEvent| ev.stop_propagation(),
        value = content,
        change = move |ev| on_change(&ev, &editing_mut, &content_mut),
        blur = move |ev: FocusEvent| on_change(ev.as_ref(), &editing_mut2, &content_mut2),
        after_render = |element| {
            let Some(input) = element.dyn_ref::<HtmlInputElement>() else {
                warn!("Not an <input> tag!");
                return;
            };
            let focused = input.focus();
            focused.unwrap_or_else(|error| warn!("Failed to focus input text: {error:?}"));
        }
    )
}

#[html]
#[template]
fn show_printed<P, PI>(
    editable: XSignal<bool>,
    printed: impl FnOnce() -> PI + Clone + 'static,
    editing_mut: MutableSignal<bool>,
) where
    XNode: From<P>,
    PI: IntoIterator<Item = P>,
{
    span(
        class = style::printed,
        click = move |ev: MouseEvent| {
            if editable.get_value_untracked() {
                ev.stop_propagation();
                editing_mut.set(true);
            }
        },
        printed()..,
    )
}

fn on_change(ev: &Event, editing_mut: &MutableSignal<bool>, content_mut: &MutableSignal<XString>) {
    let _batch = Batch::use_batch(EDITABLE_ELEMENT);
    editing_mut.set(false);
    let Ok(current_target): Result<HtmlInputElement, _> = ev
        .current_target_element(EDITABLE_ELEMENT)
        .map_err(|error| warn!("{error}"))
    else {
        return;
    };
    content_mut.set(current_target.value())
}
