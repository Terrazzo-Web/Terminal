#![cfg(feature = "client")]
#![deny(unused_crate_dependencies)]

use std::rc::Rc;
use std::sync::Mutex;

use terrazzo::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

mod api;
mod terminal;
mod terminal_id;
use terrazzo::widgets;

#[wasm_bindgen]
pub fn start() {
    terrazzo::setup_logging();
    tracing::info!("Starting client");

    let window = web_sys::window().or_throw("window");
    let document = window.document().or_throw("document");
    self::widgets::resize_event::ResizeEvent::set_up(&window);

    let main = document
        .get_element_by_id("main")
        .or_throw("#main not found");
    let main = XTemplate::new(Rc::new(Mutex::new(main)));
    let () = terminal::terminals(main);
}
