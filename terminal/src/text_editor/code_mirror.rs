#![cfg(feature = "client")]

use terrazzo::prelude::Closure;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::Element;

#[wasm_bindgen(module = "/src/text_editor/code_mirror.js")]
extern "C" {
    #[derive(Clone)]
    pub type CodeMirrorJs;

    #[wasm_bindgen(constructor)]
    pub fn new(
        element: Element,
        content: JsValue,
        onchange: &Closure<dyn FnMut(JsValue)>,
        base_path: String,
        full_path: String,
    ) -> CodeMirrorJs;

    #[wasm_bindgen(method)]
    pub fn set_content(this: &CodeMirrorJs, content: String);
}
