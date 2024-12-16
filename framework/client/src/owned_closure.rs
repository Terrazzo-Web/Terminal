use std::cell::RefCell;
use std::sync::Arc;

use named::named;
use named::NamedEnumValues;
use named::NamedType;
use wasm_bindgen::closure::IntoWasmClosure;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Function;

/// A Javascript function that owns the backing Rust callback;
///
/// When a [Closure] is turned into a Javascript [Function], the [Function]
/// gets invalidated when the [Closure] is dropped. This goes against Rust's
/// no use-after-free guarantee.
///
/// An [XOwnedClosure] creates functions that hold an [Arc] to the [Closure],
/// essentially 'leaking' the [Closure] into the [Function] and ensuring the
/// [Closure] isn't dropped until the refcount drops to zero.
pub struct XOwnedClosure<F: ?Sized>(Arc<RefCell<Option<Closure<F>>>>);

macro_rules! impl_owned_callback {
    (
        new_fn: $new_fn:ident,
        arg_types: [ $($arg_types:ty),* ],
        arg_names: [ $($arg_names:ident),* ]
    ) => {
        impl XOwnedClosure<dyn Fn($($arg_types),*)> {
            pub fn $new_fn<FT: FnOnce(Box<dyn Fn() -> Result<(), CallbackSelfDropError>>) -> F, F>(f: FT) -> Self
            where
                F: Fn($($arg_types),*) + IntoWasmClosure<dyn Fn($($arg_types),*)> + 'static,
            {
                let self_ref: Arc<RefCell<Option<Closure<dyn Fn($($arg_types),*)>>>> = Default::default();
                let closure = Closure::new(f(make_drop_self_ref_fn(self_ref.clone())));
                *self_ref.borrow_mut() = Some(closure);
                Self(self_ref)
            }
        }
    };
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum CallbackSelfDropError {
    #[error("[{}][{}] The callback was already dropped!", Self::type_name(), self.name())]
    AlreadyDropped,
}

fn make_drop_self_ref_fn<T: 'static>(
    self_ref: Arc<RefCell<Option<T>>>,
) -> Box<dyn Fn() -> Result<(), CallbackSelfDropError>> {
    Box::new(move || {
        let self_ref = self_ref.clone();
        self_ref
            .take()
            .map(|_| ())
            .ok_or(CallbackSelfDropError::AlreadyDropped)
    })
}

impl<F: ?Sized> XOwnedClosure<F> {
    pub fn as_function(&self) -> Option<Function> {
        let closure_ref = self.0.borrow();
        let maybe_closure = &*closure_ref;
        let closure = maybe_closure.as_ref()?;
        let js_value: &JsValue = closure.as_ref();
        js_value.dyn_ref().cloned()
    }
}

mod implementations {
    use super::*;

    impl_owned_callback! {
        new_fn: new,
        arg_types: [],
        arg_names: []
    }

    impl_owned_callback! {
        new_fn: new1,
        arg_types: [JsValue],
        arg_names: [a1]
    }

    impl_owned_callback! {
        new_fn: new2,
        arg_types: [JsValue, JsValue],
        arg_names: [a1, a2]
    }

    impl_owned_callback! {
        new_fn: new3,
        arg_types: [JsValue, JsValue, JsValue],
        arg_names: [a1, a2, a3]
    }

    impl_owned_callback! {
        new_fn: new4,
        arg_types: [JsValue, JsValue, JsValue, JsValue],
        arg_names: [a1, a2, a3, a4]
    }

    impl_owned_callback! {
        new_fn: new5,
        arg_types: [JsValue, JsValue, JsValue, JsValue, JsValue],
        arg_names: [a1, a2, a3, a4, a5]
    }
}
