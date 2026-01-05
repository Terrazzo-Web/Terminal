#![cfg(feature = "client")]

use std::num::NonZero;
use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Event;

use self::diagnostics::warn;
use super::style;
use crate::assets::icons;

/// State shows a spinner when the file is being saved.
#[derive(Clone)]
pub enum SynchronizedState {
    Sync,
    Pending {
        count: NonZero<u32>,
        before_unload: Arc<UiThreadSafe<BeforeUnloadCallback>>,
    },
}

impl std::fmt::Debug for SynchronizedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync => write!(f, "Sync"),
            Self::Pending { count, .. } => f.debug_struct("Pending").field("count", count).finish(),
        }
    }
}

#[html]
#[template(tag = img)]
pub fn show_synchronized_state(synchronized_state: XSignal<SynchronizedState>) -> XElement {
    tag(
        class = style::sync_status,
        src %= icon_src(synchronized_state.clone()),
    )
}

#[template(wrap = true)]
fn icon_src(#[signal] synchronized_state: SynchronizedState) -> XAttributeValue {
    match synchronized_state {
        SynchronizedState::Sync => icons::done(),
        SynchronizedState::Pending { .. } => icons::loading(),
    }
}

impl SynchronizedState {
    pub fn enqueue(state: XSignal<SynchronizedState>) -> impl Drop + Send + Sync {
        state.update(|state| {
            Some(match state {
                Self::Sync => Self::Pending {
                    count: NonZero::<u32>::MIN,
                    before_unload: set_beforeunload_event().into(),
                },
                Self::Pending {
                    count,
                    before_unload,
                } => Self::Pending {
                    count: count.saturating_add(1),
                    before_unload: before_unload.clone(),
                },
            })
        });
        scopeguard::guard(state, |state| {
            state.update(|state| {
                Some(match state {
                    Self::Sync => {
                        warn!("Impossible state");
                        Self::Sync
                    }
                    Self::Pending {
                        count,
                        before_unload,
                    } => match (count.get() - 1).try_into() {
                        Ok(count) => Self::Pending {
                            count,
                            before_unload: before_unload.clone(),
                        },
                        Err(_zero) => {
                            unset_beforeunload_event(before_unload);
                            Self::Sync
                        }
                    },
                })
            });
        })
    }
}

const BEFORE_UNLOAD: &str = "beforeunload";
type BeforeUnloadCallback = Closure<dyn Fn(Event) -> JsValue>;

fn set_beforeunload_event() -> UiThreadSafe<BeforeUnloadCallback> {
    let window = web_sys::window().or_throw("window");
    let listener: Closure<dyn Fn(Event) -> JsValue> = Closure::new(|event: Event| {
        warn!("Prevent closing window while there are pending changes");
        event.prevent_default();
        return JsValue::from_str("There are pending changes.");
    });
    let () = window
        .add_event_listener_with_callback(BEFORE_UNLOAD, listener.as_ref().unchecked_ref())
        .unwrap_or_else(|error| warn!("Failed to register {BEFORE_UNLOAD} event: {error:?}"));
    return UiThreadSafe::from(listener);
}

fn unset_beforeunload_event(listener: &BeforeUnloadCallback) {
    let window = web_sys::window().or_throw("window");
    let () = window
        .remove_event_listener_with_callback(BEFORE_UNLOAD, listener.as_ref().unchecked_ref())
        .unwrap_or_else(|error| warn!("Failed to unregister {BEFORE_UNLOAD} event: {error:?}"));
}
