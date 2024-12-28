#![cfg(feature = "client")]
#![deny(unused_crate_dependencies)]

use std::rc::Rc;
use std::sync::Mutex;

use terrazzo_client::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

mod api;
mod terminal;
mod terminal_id;
mod widgets;

#[wasm_bindgen]
pub fn start() {
    terrazzo_client::setup_logging();
    let () = start_impl().unwrap();
}

fn start_impl() -> Option<()> {
    tracing::info!("Starting client");
    let window = web_sys::window()?;
    let document = window.document()?;
    self::widgets::resize_event::ResizeEvent::set_up(&window);

    let main = document.get_element_by_id("main")?;
    let main = XTemplate::new(Rc::new(Mutex::new(main)));
    let () = terminal::terminals(main);
    Some(())
}
