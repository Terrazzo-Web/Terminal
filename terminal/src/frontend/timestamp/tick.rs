use std::sync::Mutex;
use std::time::Duration;

use nameth::NamedType;
use nameth::nameth;
use terrazzo::prelude::*;
use web_sys::window;

use self::diagnostics::debug;
use super::datetime::DateTime;

/// A wrapper for the [Closure] and the interval timer handle ID.
#[nameth]
#[derive(Clone)]
pub struct Tick(Ptr<Mutex<TickInner>>);

struct TickInner {
    period: Duration,
    now: DateTime,
    on_drop: Option<AbortTickOnDrop>,
}

pub struct AbortTickOnDrop {
    #[expect(unused)]
    pub closure: Closure<dyn Fn()>,
    pub handle: i32,
}

impl Tick {
    pub fn new(period: Duration) -> Self {
        Self(Ptr::new(Mutex::new(TickInner {
            period,
            now: DateTime::now(),
            on_drop: None,
        })))
    }

    pub fn now(&self) -> DateTime {
        self.0.lock().unwrap().now.clone()
    }

    pub fn tick(&self) {
        self.0.lock().unwrap().now = DateTime::now();
    }

    pub fn set_on_drop(&self, abort_tick_on_drop: AbortTickOnDrop) {
        self.0.lock().unwrap().on_drop = Some(abort_tick_on_drop)
    }
}

impl std::fmt::Debug for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tick = self.0.lock().unwrap();
        f.debug_struct(Tick::type_name())
            .field("period", &tick.period)
            .field("now", &tick.now)
            .finish()
    }
}

impl Drop for TickInner {
    fn drop(&mut self) {
        debug!("Drop timer for period={:?}", self.period);
        let Some(AbortTickOnDrop { handle, .. }) = &self.on_drop else {
            return;
        };
        let window = window().unwrap();
        window.clear_interval_with_handle(*handle);
    }
}
