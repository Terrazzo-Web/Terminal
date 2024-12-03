use std::cell::OnceCell;
use std::time::Duration;

use terrazzo_client::owned_closure;
use terrazzo_client::prelude::*;
use tracing::debug;
use wasm_bindgen::JsValue;
use web_sys::Window;

use super::debounce::DoDebounce as _;

pub struct ResizeEvent(OnceCell<XSignal<()>>);

impl ResizeEvent {
    pub fn signal() -> &'static XSignal<()> {
        static SIGNAL: ResizeEvent = ResizeEvent(OnceCell::new());
        SIGNAL.0.get_or_init(|| XSignal::new("ResizeEvent", ()))
    }

    pub fn set_up(window: &Window) {
        let closure = owned_closure::XOwnedClosure::new1(|self_drop| {
            Duration::from_millis(200).debounce(move |resize_event: JsValue| {
                let _self_drop = &self_drop;
                debug!("Window resized: {resize_event:?}");
                ResizeEvent::signal().force(())
            })
        });
        let function = closure.as_function().or_throw("as_function()");
        window
            .add_event_listener_with_callback("resize", &function)
            .or_throw("add_event_listener");
    }
}

unsafe impl Sync for ResizeEvent {}
