#![cfg(feature = "client")]

use std::sync::Mutex;

use terrazzo::prelude::*;
use terrazzo::widgets;
use wasm_bindgen::prelude::wasm_bindgen;

use self::login::login;

pub mod login;
pub mod menu;
pub mod utils;

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
    let main = XTemplate::new(Ptr::new(Mutex::new(main)));
    let () = ui(main);
}

fn ui(main: XTemplate) {
    let consumers = login(main, login::logged_in());
    std::mem::forget(consumers);
}
