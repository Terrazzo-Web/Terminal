#![allow(unused)]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Days;
use chrono::NaiveDate;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use nameth::NamedType;
use nameth::nameth;
use terrazzo::autoclone;
use terrazzo::prelude::*;
use tracing::debug;
use tracing::warn;
use wasm_bindgen::JsCast;
use web_sys::window;

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

/// Returns a signal that updates every second.
///
/// There is only ever one instance of the [second_timer].
/// We keep a static weak reference to the timer to ensure we keep using the
/// same instance until all references are dropped.
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
    let timer = XSignal::new(
        "second-timer",
        Tick(Arc::new(Mutex::new(TickInner {
            period,
            now: Utc::now(),
            on_drop: None,
        }))),
    );
    let timer_weak = timer.downgrade();
    let closure: Closure<dyn Fn()> = Closure::new(move || {
        let Some(timer) = timer_weak.upgrade() else {
            return;
        };
        let tick = timer.get_value_untracked();
        tick.0.lock().unwrap().now = Utc::now();
        timer.force(tick)
    });
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
    tick.0.lock().unwrap().on_drop = Some(AbortTickOnDrop { closure, handle });

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

/// A wrapper for the [Closure] and the handle ID.
#[nameth]
#[derive(Clone)]
struct Tick(Arc<Mutex<TickInner>>);

struct TickInner {
    period: Duration,
    now: DateTime<Utc>,
    on_drop: Option<AbortTickOnDrop>,
}

struct AbortTickOnDrop {
    closure: Closure<dyn Fn()>,
    handle: i32,
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

impl std::fmt::Debug for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tick = self.0.lock().unwrap();
        f.debug_struct(Tick::type_name())
            .field("period", &tick.period)
            .field("now", &tick.now)
            .finish()
    }
}

/// Creates a signal that produces a friendly representation of a timetamp.
#[autoclone]
pub fn display_timestamp<TZ: TimeZone + 'static>(value: DateTime<TZ>) -> XSignal<Timestamp<TZ>> {
    let timer_mode_signal = XSignal::new("timer-mode", TimerMode::AbsoluteTime);
    let timestamp_signal = XSignal::new(
        "display_timetamp",
        Timestamp {
            timer_mode_signal: timer_mode_signal.clone(),
            timer_mode_consumers: None,
            timer_consumers: None,
            display: String::new(),
            value,
        },
    );

    timestamp_signal.update_mut(move |timestamp| {
        autoclone!(timestamp_signal);

        // Subscribe to timer mode changes.
        let timer_mode_consumers = timer_mode_signal.add_subscriber(move |timer_mode| {
            let timer_consumers = if let Some(timer) = timer_mode.timer() {
                // Subscribe to the timer.
                Some(timer.add_subscriber(move |_tick| {
                    // On tick
                    autoclone!(timer_mode, timestamp_signal);
                    timestamp_signal.update_mut(|timestamp| timestamp.recompute(&timer_mode))
                }))
            } else {
                None
            };

            // Record the ticks event.
            timestamp_signal.update_mut(|timestamp| Timestamp {
                timer_consumers,
                ..timestamp.take()
            });
        });

        // Record the timer mode event.
        Timestamp {
            timer_mode_consumers: Some(timer_mode_consumers),
            ..timestamp.take()
        }
    });
    return timestamp_signal;
}

/// Represents a printable timestamp.
///
/// The string representation is computed to
/// - an intuitive representation of some time ago for recent timestamps
/// - a formal timstamp for older timestamps
pub struct Timestamp<TZ: TimeZone> {
    /// A signal that indicates how the timestamp should be printed.
    /// As the timestamp becomes older, the [TimerMode] will change.
    timer_mode_signal: XSignal<TimerMode>,

    /// Holds a reference to the closure that reacts to timer mode changes.
    timer_mode_consumers: Option<Consumers>,

    /// Holds a reference to the closure that reacts to timer ticks.
    /// The timer depends on the timer mode.
    timer_consumers: Option<Consumers>,

    /// The display value of the timestamp.
    display: String,

    /// The formal value of the timestamp.
    value: DateTime<TZ>,
}

impl<TZ: TimeZone> Timestamp<TZ> {
    fn recompute(&mut self, timer_mode: &TimerMode) -> Self {
        let now = timer_mode
            .now()
            .map(|now| now.with_timezone(&self.value.timezone()));

        if let Some(now) = &now {
            {
                let ago = now.clone() - self.value.clone();
                if ago <= chrono::Duration::minutes(30) {
                    self.timer_mode_signal
                        .set(TimerMode::MomentsAgo(minute_timer()));
                    return Self {
                        display: print_ago(ago),
                        ..self.take()
                    };
                }
            }

            if let (Some(now_start_of_day), Some(timestamp_start_of_day)) =
                (from_ymd_opt(&now), from_ymd_opt(&self.value))
            {
                if timestamp_start_of_day == now_start_of_day {
                    self.timer_mode_signal
                        .set(TimerMode::DaysAgo(minute_timer()));
                    return Self {
                        display: format!("Today, {}", hour_minute(&self.value)),
                        ..self.take()
                    };
                }
                if Some(timestamp_start_of_day) == now_start_of_day.checked_sub_days(Days::new(1)) {
                    self.timer_mode_signal
                        .set(TimerMode::DaysAgo(minute_timer()));
                    return Self {
                        display: format!("Yesterday, {}", hour_minute(&self.value)),
                        ..self.take()
                    };
                }
            }
        }

        self.timer_mode_signal.set(TimerMode::AbsoluteTime);
        return Self {
            display: format!(
                "{}, {}",
                day_month_year(&self.value),
                hour_minute(&self.value)
            ),
            ..self.take()
        };
    }

    fn take(&mut self) -> Self {
        let timezone = self.value.timezone();
        Self {
            timer_mode_signal: self.timer_mode_signal.clone(),
            timer_mode_consumers: self.timer_mode_consumers.take(),
            timer_consumers: self.timer_mode_consumers.take(),
            display: std::mem::take(&mut self.display),
            value: std::mem::replace(
                &mut self.value,
                DateTime::UNIX_EPOCH.with_timezone(&timezone),
            ),
        }
    }
}

impl<TZ: TimeZone> std::fmt::Display for Timestamp<TZ> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display.fmt(f)
    }
}

impl<TZ: TimeZone> std::fmt::Debug for Timestamp<TZ> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timestamp")
            .field("timer_mode_consumers", &self.timer_mode_consumers.is_some())
            .field("timer_consumers", &self.timer_consumers.is_some())
            .field("value", &self.display)
            .finish()
    }
}

#[derive(Clone, Debug)]
enum TimerMode {
    MomentsAgo(Timer),
    DaysAgo(Timer),
    AbsoluteTime,
}

impl PartialEq for TimerMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MomentsAgo { .. }, Self::MomentsAgo { .. })
            | (Self::DaysAgo { .. }, Self::DaysAgo { .. })
            | (Self::AbsoluteTime, Self::AbsoluteTime) => true,
            _ => false,
        }
    }
}

impl Eq for TimerMode {}

impl TimerMode {
    fn now(&self) -> Option<DateTime<Utc>> {
        self.timer()
            .map(|timer| timer.get_value_untracked().0.lock().unwrap().now.clone())
    }

    fn timer(&self) -> Option<Timer> {
        if let TimerMode::MomentsAgo(timer) | TimerMode::DaysAgo(timer) = self {
            Some(timer.clone())
        } else {
            None
        }
    }
}

fn print_ago(mut ago: chrono::Duration) -> String {
    let hours = ago.num_hours();
    ago = ago - chrono::Duration::hours(hours);
    let minutes = ago.num_minutes();
    ago = ago - chrono::Duration::minutes(minutes);
    let seconds = ago.num_seconds();
    if hours != 0 {
        return format!("{:0>2}:{:0>2}:{:0>2}s ago", hours, minutes, seconds);
    }
    if minutes != 0 {
        return format!("{:0>2}:{:0>2}s ago", minutes, seconds);
    }
    return format!("{:0>2}s ago", seconds);
}

fn from_ymd_opt(timestamp: &chrono::DateTime<impl TimeZone>) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(timestamp.year(), timestamp.month(), timestamp.day())
}

fn hour_minute(timestamp: &chrono::DateTime<impl TimeZone>) -> String {
    format!("{:0>2}:{:0>2}", timestamp.hour(), timestamp.minute())
}

fn day_month_year(timestamp: &chrono::DateTime<impl TimeZone>) -> String {
    format!(
        "{:0>2}.{:0>2}.{}",
        timestamp.day(),
        timestamp.month() as u8,
        timestamp.year()
    )
}
