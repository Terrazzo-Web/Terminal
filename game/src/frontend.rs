#![cfg(feature = "client")]

use std::rc::Rc;
use std::sync::Mutex;

use terrazzo::prelude::*;
use tracing::info;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::game;

#[wasm_bindgen]
pub fn start() {
    terrazzo::setup_logging();
    info!("Starting client");

    let window = web_sys::window().or_throw("window");
    let document = window.document().or_throw("document");
    let main = document
        .get_element_by_id("main")
        .or_throw("#main not found");

    let template = XTemplate::new(Rc::new(Mutex::new(main.clone())));
    std::mem::forget(template.clone());
    let consumer = game::run(template, main);
    std::mem::forget(consumer);
}
