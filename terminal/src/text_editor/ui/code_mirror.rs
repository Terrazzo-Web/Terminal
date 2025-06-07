use terrazzo::prelude::Closure;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::Element;

#[wasm_bindgen(module = "/src/text_editor/ui/code_mirror.js")]
extern "C" {
    #[derive(Clone)]
    pub type CodeMirrorJs;

    #[wasm_bindgen(constructor)]
    pub fn new(
        element: Element,
        content: JsValue,
        onchange: &Closure<dyn FnMut(JsValue)>,
    ) -> CodeMirrorJs;
}
