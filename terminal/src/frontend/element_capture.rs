use std::any::type_name;
use std::sync::Arc;
use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::prelude::OrElseLog;
use terrazzo::prelude::UiThreadSafe;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Element;

pub struct ElementCapture<T: AsRef<JsValue>>(Arc<OnceLock<UiThreadSafe<T>>>);

impl<T: AsRef<JsValue> + JsCast + 'static> ElementCapture<T> {
    #[autoclone]
    pub fn capture(&self) -> impl Fn(Element) + 'static {
        let this = self.clone();
        move |element| {
            autoclone!(this);
            let element: T = element
                .dyn_into::<T>()
                .or_else_throw(|e| format!("'{}' is not an '{}'", e.to_string(), type_name::<T>()));
            let element: UiThreadSafe<T> = UiThreadSafe::<T>::from(element);
            this.0.set(element).or_throw("Element already set");
        }
    }

    pub fn try_get(&self) -> Option<&T> {
        self.0.get().map(|e| &**e)
    }

    pub fn get(&self) -> &T {
        self.try_get().or_throw("Element was not set")
    }
}

impl<T: AsRef<JsValue>> std::fmt::Debug for ElementCapture<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let maybe_element = self.0.get();
        f.debug_tuple("ElementCapture")
            .field(&maybe_element.and_then(|element| element.as_ref().as_string()))
            .finish()
    }
}

impl<T: AsRef<JsValue>> Clone for ElementCapture<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: AsRef<JsValue>> Default for ElementCapture<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}
