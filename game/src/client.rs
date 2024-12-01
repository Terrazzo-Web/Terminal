#![cfg(feature = "client")]

use std::rc::Rc;
use std::sync::Mutex;

use terrazzo_client::prelude::*;
use tracing::info;
use wasm_bindgen::prelude::wasm_bindgen;

mod game;

#[wasm_bindgen]
pub fn start() {
    terrazzo_client::setup_logging();
    let () = start_impl().unwrap();
}

fn start_impl() -> Option<()> {
    info!("Starting client");
    let window = web_sys::window()?;
    let document = window.document()?;

    let main = document.get_element_by_id("main")?;
    let template = XTemplate::new(Rc::new(Mutex::new(main.clone())));
    std::mem::forget(template.clone());
    let consumer = game::run(template, main);
    std::mem::forget(consumer);
    Some(())
}
