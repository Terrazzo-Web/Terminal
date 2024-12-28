use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use scopeguard::guard;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

pub trait DoDebounce {
    fn debounce<T: 'static>(self, f: impl Fn(T) + 'static) -> impl Fn(T);
}

pub struct Debounce {
    pub delay: Duration,
    pub max_delay: Option<Duration>,
}

impl DoDebounce for Duration {
    fn debounce<T: 'static>(self, f: impl Fn(T) + 'static) -> impl Fn(T) {
        Debounce {
            delay: self,
            max_delay: Some(self),
        }
        .debounce(f)
    }
}

impl DoDebounce for Debounce {
    fn debounce<T: 'static>(self, f: impl Fn(T) + 'static) -> impl Fn(T) {
        let window = web_sys::window().expect("window");
        let performance = window.performance().expect("performance");
        let state = Rc::new(Cell::new(DebounceState::default()));
        let delay_millis = self.delay.as_secs_f64() * 1000.;
        let max_delay_millis = self.max_delay.map(|d| d.as_secs_f64() * 1000.);
        let closure: Closure<dyn Fn()> = Closure::new({
            let state = state.clone();
            let performance = performance.clone();
            move || {
                let mut state = guard(state.take(), |new_state| state.set(new_state));
                f(state.schedled_run.take().unwrap().arg);
                state.last_run = performance.now();
            }
        });
        move |arg| {
            let now = performance.now();
            let mut state = guard(state.take(), |new_state| state.set(new_state));
            if let Some(max_delay_millis) = max_delay_millis {
                if now - state.last_run - delay_millis > max_delay_millis {
                    // If max delay is exceeded and there is already a task running, let it run.
                    if let Some(schedled_run) = &mut state.schedled_run {
                        schedled_run.arg = arg;
                        return;
                    }
                }
            }

            if let Some(ScheduledRun { timeout_id, .. }) = state.schedled_run {
                window.clear_timeout_with_handle(timeout_id);
            }
            let timeout_id = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    (self.delay.as_secs() * 1000) as i32,
                )
                .expect("set_timeout");
            state.schedled_run = Some(ScheduledRun { timeout_id, arg });
        }
    }
}

struct DebounceState<T> {
    schedled_run: Option<ScheduledRun<T>>,
    last_run: f64,
}

struct ScheduledRun<T> {
    timeout_id: i32,
    arg: T,
}

impl<T> Default for DebounceState<T> {
    fn default() -> Self {
        Self {
            schedled_run: None,
            last_run: 0.,
        }
    }
}

impl<T> std::fmt::Debug for DebounceState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebounceState")
            .field("schedled_run", &self.schedled_run.is_some())
            .field("last_run", &self.last_run)
            .finish()
    }
}
