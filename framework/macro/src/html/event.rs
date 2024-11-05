use quote::quote;

pub fn process_event(name: &syn::Ident, value: &syn::Expr) -> Option<proc_macro2::TokenStream> {
    Some({
        let name = name.to_string();
        let event_type = get_event_type(&name)?;
        quote! {
            XEvent {
                event_type: #name.into(),
                callback: std::sync::Arc::new(Closure::<dyn Fn(#event_type) -> _>::new(#value)),
            }
        }
    })
}

fn get_event_type(event_name: &str) -> Option<proc_macro2::TokenStream> {
    Some(match event_name {
        "click" | "dblclick" | "mousedown" | "mouseup" | "mousemove" | "mouseover" | "mouseout"
        | "mouseenter" | "mouseleave" | "contextmenu" => quote!(web_sys::MouseEvent),
        "keydown" | "keypress" | "keyup" => quote!(web_sys::KeyboardEvent),
        "focus" | "blur" => quote!(web_sys::FocusEvent),
        "change" | "submit" => quote!(web_sys::Event),
        "input" => quote!(web_sys::InputEvent),
        "scroll" | "resize" => quote!(web_sys::UIEvent),
        "drag" | "dragstart" | "dragend" | "dragenter" | "dragleave" | "dragover" | "drop" => {
            quote!(web_sys::DragEvent)
        }
        "load" | "unload" | "abort" => quote!(web_sys::Event),
        "error" => quote!(web_sys::ErrorEvent),
        "hashchange" => quote!(web_sys::HashChangeEvent),
        "popstate" => quote!(web_sys::PopStateEvent),
        "beforeunload" => quote!(web_sys::BeforeUnloadEvent),
        "touchstart" | "touchmove" | "touchend" | "touchcancel" => quote!(web_sys::TouchEvent),
        "pointerdown" | "pointerup" | "pointermove" | "pointerenter" | "pointerleave" => {
            quote!(web_sys::PointerEvent)
        }
        "wheel" => quote!(web_sys::WheelEvent),
        "animationstart" | "animationend" | "animationiteration" => quote!(web_sys::AnimationEvent),
        "transitionstart" | "transitionend" | "transitionrun" => quote!(web_sys::TransitionEvent),
        _ => return None,
    })
}
