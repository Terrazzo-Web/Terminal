#![cfg(feature = "text-editor")]

use std::sync::Arc;
use std::time::Duration;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::autoclone;
use terrazzo::prelude::*;
use wasm_bindgen_futures::spawn_local;

use self::datetime::DateTime;
use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use self::tick::Tick;
use self::timer::Timer;
use self::timer::fraction_timer;
use self::timer::minute_timer;
use self::timer::second_timer;
use self::timer::ten_seconds_timer;

pub mod datetime;
mod tick;
mod timer;

/// Creates a signal that produces a friendly representation of a timetamp.
pub fn display_timestamp(value: DateTime) -> XSignal<Box<Timestamp>> {
    let timer_mode_signal = XSignal::new("timer-mode", TimerMode::fractions_ago());
    let timestamp_signal = XSignal::new(
        "display-timetamp",
        Timestamp {
            display: Arc::default(),
            inner: Ptr::new(TimestampInner {
                timer_mode_signal: timer_mode_signal.clone(),
                timer_mode_consumers: None.into(),
                timer_consumers: None.into(),
                value,
            }),
        }
        .recompute(&TimerMode::fractions_ago()),
    );

    timestamp_signal.update(setup_display_timestamp_signals(
        timestamp_signal.downgrade(),
        timer_mode_signal.downgrade(),
    ));

    timer_mode_signal.set(TimerMode::moments_ago());
    debug! { "timestamp_signal = {:?}", timestamp_signal.get_value_untracked() };
    return timestamp_signal;
}

fn setup_display_timestamp_signals(
    timestamp_signal_weak: XSignalWeak<Box<Timestamp>>,
    timer_mode_signal_weak: XSignalWeak<TimerMode>,
) -> impl FnOnce(&Box<Timestamp>) -> Option<Box<Timestamp>> {
    move |timestamp| {
        let timer_mode_signal = timer_mode_signal_weak.upgrade();
        let timer_mode_consumers =
            timer_mode_signal?.add_subscriber(setup_timer_mode_signal(&timestamp_signal_weak));

        // Record the timer mode event.
        return Some(Box::new(Timestamp {
            display: timestamp.display.clone(),
            inner: Ptr::new(TimestampInner {
                timer_mode_consumers: Ptr::new(Some(timer_mode_consumers)),
                ..timestamp.inner.as_ref().clone()
            }),
        }));
    }
}

#[autoclone]
fn setup_timer_mode_signal(
    timestamp_signal_weak: &XSignalWeak<Box<Timestamp>>,
) -> impl Fn(TimerMode) + 'static {
    move |timer_mode| {
        autoclone!(timestamp_signal_weak);
        debug!("Update timer_mode to {timer_mode:?}");
        let timer_consumers = timer_mode.timer().map(|timer| {
            timer.add_subscriber(setup_timer_signal(&timestamp_signal_weak, timer_mode))
        });

        let Some(timestamp_signal) = timestamp_signal_weak.upgrade() else {
            return;
        };
        let update_timestamp_signal_async = async move {
            timestamp_signal.update_mut(move |timestamp| {
                Box::new(Timestamp {
                    display: std::mem::take(&mut timestamp.display),
                    inner: Ptr::new(TimestampInner {
                        timer_consumers: Ptr::new(timer_consumers),
                        ..timestamp.inner.as_ref().clone()
                    }),
                })
            })
        };
        spawn_local(update_timestamp_signal_async.in_current_span());
    }
}

#[autoclone]
fn setup_timer_signal(
    timestamp_signal_weak: &XSignalWeak<Box<Timestamp>>,
    timer_mode: TimerMode,
) -> impl Fn(Tick) + 'static {
    move |_tick| {
        autoclone!(timer_mode, timestamp_signal_weak);
        let Some(timestamp_signal) = timestamp_signal_weak.upgrade() else {
            return;
        };
        let update_timestamp_signal_async = async move {
            autoclone!(timer_mode);
            timestamp_signal.update_mut(|timestamp| timestamp.recompute(&timer_mode))
        };
        spawn_local(update_timestamp_signal_async.in_current_span())
    }
}

/// Represents a printable timestamp.
///
/// The string representation is computed to
/// - an intuitive representation of some time ago for recent timestamps
/// - a formal timstamp for older timestamps
#[derive(Clone)]
pub struct Timestamp {
    /// The display value of the timestamp.
    display: Arc<str>,

    inner: Ptr<TimestampInner>,
}

#[derive(Clone)]
struct TimestampInner {
    /// A signal that indicates how the timestamp should be printed.
    /// As the timestamp becomes older, the [TimerMode] will change.
    timer_mode_signal: XSignal<TimerMode>,

    /// Holds a reference to the closure that reacts to timer mode changes.
    timer_mode_consumers: Ptr<Option<Consumers>>,

    /// Holds a reference to the closure that reacts to timer ticks.
    /// The timer depends on the timer mode.
    timer_consumers: Ptr<Option<Consumers>>,

    /// The formal value of the timestamp.
    value: DateTime,
}

impl Timestamp {
    #[allow(unused)]
    pub fn value(&self) -> DateTime {
        self.inner.value.clone()
    }

    fn recompute(&mut self, timer_mode: &TimerMode) -> Box<Self> {
        let printed = self.print(timer_mode).into();
        return Box::new(Self {
            display: printed,
            inner: self.inner.clone(),
        });
    }

    fn print(&mut self, timer_mode: &TimerMode) -> String {
        let timestamp = &self.inner.value;
        let now = timer_mode.now();

        if let Some(now) = &now {
            if let Ok(ago) = now - timestamp {
                if ago < Duration::from_secs(15) {
                    self.inner.timer_mode_signal.set(TimerMode::fractions_ago());
                    return print_fractions_ago(ago);
                }
                if ago <= Duration::from_secs(5 * 60) {
                    self.inner.timer_mode_signal.set(TimerMode::moments_ago());
                    return print_ago(ago);
                }
                if ago <= Duration::from_secs(3600) {
                    self.inner.timer_mode_signal.set(TimerMode::minutes_ago());
                    return print_ago(ago);
                }
            }

            {
                let timestamp_start_of_day = timestamp.to_start_of_day();

                let today_start_of_day = now.to_start_of_day();
                if timestamp_start_of_day == today_start_of_day {
                    self.inner.timer_mode_signal.set(TimerMode::days_ago());
                    return format!("Today, {}", timestamp.hour_minute());
                }

                let yesterday_start_of_day = &today_start_of_day - Duration::from_secs(3600 * 24);
                if timestamp_start_of_day == yesterday_start_of_day {
                    self.inner.timer_mode_signal.set(TimerMode::days_ago());
                    return format!("Yesterday, {}", timestamp.hour_minute());
                }
            }
        }

        self.inner.timer_mode_signal.set(TimerMode::AbsoluteTime);
        return format!(
            "{}, {}",
            timestamp.day_month_year(),
            timestamp.hour_minute()
        );
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display.fmt(f)
    }
}

impl std::fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timestamp")
            .field("display", &self.display)
            .field("timer_mode", &self.inner.timer_mode_consumers.is_some())
            .field("timer", &self.inner.timer_consumers.is_some())
            .field("value", &self.inner.value)
            .finish()
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.display == other.display && self.inner.value == other.inner.value
    }
}

impl Eq for Timestamp {}

#[nameth]
#[derive(Clone, Debug)]
enum TimerMode {
    FractionsAgo(Timer),
    MomentsAgo(Timer),
    MinutesAgo(Timer),
    DaysAgo(Timer),
    AbsoluteTime,
}

impl PartialEq for TimerMode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.name(), other.name())
    }
}

impl Eq for TimerMode {}

impl TimerMode {
    fn fractions_ago() -> Self {
        Self::FractionsAgo(fraction_timer())
    }

    fn moments_ago() -> Self {
        Self::MomentsAgo(second_timer())
    }

    fn minutes_ago() -> Self {
        Self::MinutesAgo(ten_seconds_timer())
    }

    fn days_ago() -> Self {
        Self::DaysAgo(minute_timer())
    }

    fn now(&self) -> Option<DateTime> {
        self.timer().map(|timer| timer.get_value_untracked().now())
    }

    fn timer(&self) -> Option<Timer> {
        if let TimerMode::FractionsAgo(timer)
        | TimerMode::MomentsAgo(timer)
        | TimerMode::MinutesAgo(timer)
        | TimerMode::DaysAgo(timer) = self
        {
            Some(timer.clone())
        } else {
            None
        }
    }
}

fn print_fractions_ago(ago: Duration) -> String {
    let seconds = ago.as_secs();
    let millis = ago.subsec_millis();
    return format!("{:0>2}.{:0>3}s ago", seconds, millis);
}

fn print_ago(mut ago: Duration) -> String {
    let hours = ago.as_secs() / 3600;
    ago -= Duration::from_secs(hours * 3600);
    let minutes = ago.as_secs() / 60;
    ago -= Duration::from_secs(minutes * 60);
    let seconds = ago.as_secs();
    if hours != 0 {
        return format!("{hours}h {minutes}m {seconds}s ago");
    }
    if minutes != 0 {
        return format!("{minutes}m {seconds}s ago");
    }
    return format!("{seconds}s ago");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[test]
    fn print_ago() {
        assert_eq!(
            ["3h 4m 5s ago", "35m 15s ago", "15s ago"],
            [
                super::print_ago(Duration::from_secs(3 * 3600 + 4 * 60 + 5)),
                super::print_ago(Duration::from_secs(35 * 60 + 15)),
                super::print_ago(Duration::from_secs(15))
            ]
        )
    }

    #[test]
    fn print_fractions_ago() {
        assert_eq!(
            "15.345s ago",
            super::print_fractions_ago(Duration::from_millis(15 * 1000 + 345))
        )
    }
}
