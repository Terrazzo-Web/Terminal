use std::sync::Mutex;
use std::time::Duration;

use terrazzo::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::window;

use self::diagnostics::debug;
use self::diagnostics::warn;
use super::tick::AbortTickOnDrop;
use super::tick::Tick;

/// Represents a signal that updates at regular intervals.
pub type Timer = XSignal<Tick>;

/// Returns a signal that updates every second.
///
/// There is only ever one instance of the [second_timer].
/// We keep a static weak reference to the timer to ensure we keep using the
/// same instance until all references are dropped.
pub fn second_timer() -> Timer {
    static TIMER: Mutex<WeakTimer> = Mutex::new(WeakTimer(XSignalWeak::new()));
    create_timer(&TIMER, Duration::from_secs(1))
}

/// Returns a signal that updates every minute.
pub fn ten_seconds_timer() -> Timer {
    static TIMER: Mutex<WeakTimer> = Mutex::new(WeakTimer(XSignalWeak::new()));
    create_timer(&TIMER, Duration::from_secs(10))
}

/// Returns a signal that updates every fraction of a second.
pub fn fraction_timer() -> Timer {
    static TIMER: Mutex<WeakTimer> = Mutex::new(WeakTimer(XSignalWeak::new()));
    create_timer(&TIMER, Duration::from_millis(50))
}

/// Returns a signal that updates every minute.
pub fn minute_timer() -> Timer {
    static TIMER: Mutex<WeakTimer> = Mutex::new(WeakTimer(XSignalWeak::new()));
    create_timer(&TIMER, Duration::from_secs(60))
}

fn create_timer(timer: &Mutex<WeakTimer>, period: Duration) -> Timer {
    let mut lock = timer.lock().unwrap();
    if let Some(timer) = lock.0.upgrade() {
        return timer;
    }
    let timer = create_timer_impl(period);
    *lock = WeakTimer(timer.downgrade());
    return timer;
}

fn create_timer_impl(period: Duration) -> Timer {
    debug!("Create timer for period={period:?}");
    let timer = Timer::new("second-timer", Tick::new(period));
    let timer_weak = timer.downgrade();

    let closure: Closure<dyn Fn()> = Closure::new(move || {
        let Some(timer) = timer_weak.upgrade() else {
            warn!("MISSING TIMER");
            return;
        };

        debug!(?period, "Update tick.now and force trigger the signal");
        let tick = timer.get_value_untracked();
        tick.tick();
        timer.force(tick)
    });

    // Create the interval timer.
    let window = window().unwrap();
    let Ok(handle) = window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        period.as_millis() as i32,
    ) else {
        warn!("Can't create interval timer");
        return timer;
    };

    // Record the closure and the handle inside the Tick.
    // When the signal drops, the tick drops, and the interval timer is canceled.
    let tick = timer.get_value_untracked();
    tick.set_on_drop(AbortTickOnDrop { closure, handle });

    return timer;
}

/// A weak reference to the timer.
///
/// The static variable and the closure contain weak references.
///
/// Only places that actually use the timer need strong references.
struct WeakTimer(XSignalWeak<Tick>);

unsafe impl Send for WeakTimer {}
unsafe impl Sync for WeakTimer {}
