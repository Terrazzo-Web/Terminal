#![cfg(feature = "client")]

use std::rc::Rc;
use std::sync::Mutex;

use terrazzo::prelude::*;
use terrazzo::widgets;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::terminal::terminals;

#[wasm_bindgen]
pub fn start() {
    terrazzo::setup_logging();
    tracing::info!("Starting client");

    let window = web_sys::window().or_throw("window");
    let document = window.document().or_throw("document");
    widgets::resize_event::ResizeEvent::set_up(&window);

    let main = document
        .get_element_by_id("main")
        .or_throw("#main not found");
    let main = XTemplate::new(Rc::new(Mutex::new(main)));
    let () = terminals(main);
}
