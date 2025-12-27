use std::any::type_name;
use std::sync::Arc;
use std::sync::Mutex;

use terrazzo::autoclone;
use terrazzo::prelude::OrElseLog;
use terrazzo::prelude::UiThreadSafe;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Element;

pub struct ElementCapture<T: AsRef<JsValue>>(Arc<Mutex<Option<UiThreadSafe<T>>>>);

impl<T: AsRef<JsValue> + JsCast + 'static> ElementCapture<T> {
    #[autoclone]
    pub fn capture(&self) -> impl Fn(Element) + 'static {
        let this = self.clone();
        let on_drop = scopeguard::guard((), move |()| {
            autoclone!(this);
            *this.lock() = None;
        });
        move |element| {
            let _ = &on_drop;
            let element: T = element
                .dyn_into::<T>()
                .or_else_throw(|e| format!("'{}' is not an '{}'", e.to_string(), type_name::<T>()));
            this.try_set(element).or_throw("Element already set");
        }
    }
}

impl<T: AsRef<JsValue>> ElementCapture<T> {
    pub fn try_with<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.lock().as_ref().map(|value| f(&*value))
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.try_with(f).or_throw("Element was not set")
    }

    fn try_set(&self, element: T) -> Result<(), ()> {
        let mut lock = self.lock();
        if lock.is_some() {
            return Err(());
        }
        *lock = Some(element.into());
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<UiThreadSafe<T>>> {
        self.0.lock().or_throw("lock ElementCapture")
    }
}

impl<T: AsRef<JsValue>> std::fmt::Debug for ElementCapture<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let maybe_element = self.try_with(|value| format!("{:?}", value.as_ref()));
        f.debug_tuple("ElementCapture")
            .field(&maybe_element)
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
