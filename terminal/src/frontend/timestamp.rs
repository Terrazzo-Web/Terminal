use std::sync::Arc;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Days;
use chrono::NaiveDate;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::autoclone;
use terrazzo::prelude::*;

use self::diagnostics::debug;
use self::tick::Tick;
use self::timer::Timer;
use self::timer::fraction_timer;
use self::timer::minute_timer;
use self::timer::second_timer;
use self::timer::ten_seconds_timer;

pub mod tick;
pub mod timer;

/// Creates a signal that produces a friendly representation of a timetamp.
pub fn display_timestamp<TZ: TimeZone + 'static>(
    value: DateTime<TZ>,
) -> XSignal<Box<Timestamp<TZ>>> {
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

fn setup_display_timestamp_signals<TZ: TimeZone + 'static>(
    timestamp_signal_weak: XSignalWeak<Box<Timestamp<TZ>>>,
    timer_mode_signal_weak: XSignalWeak<TimerMode>,
) -> impl FnOnce(&Box<Timestamp<TZ>>) -> Option<Box<Timestamp<TZ>>> {
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
fn setup_timer_mode_signal<TZ: TimeZone + 'static>(
    timestamp_signal_weak: &XSignalWeak<Box<Timestamp<TZ>>>,
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
        wasm_bindgen_futures::spawn_local(async move {
            timestamp_signal.update_mut(move |timestamp| {
                Box::new(Timestamp {
                    display: std::mem::take(&mut timestamp.display),
                    inner: Ptr::new(TimestampInner {
                        timer_consumers: Ptr::new(timer_consumers),
                        ..timestamp.inner.as_ref().clone()
                    }),
                })
            })
        });
    }
}

#[autoclone]
fn setup_timer_signal<TZ: TimeZone + 'static>(
    timestamp_signal_weak: &XSignalWeak<Box<Timestamp<TZ>>>,
    timer_mode: TimerMode,
) -> impl Fn(Tick) + 'static {
    move |_tick| {
        autoclone!(timer_mode, timestamp_signal_weak);
        let Some(timestamp_signal) = timestamp_signal_weak.upgrade() else {
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            autoclone!(timer_mode);
            timestamp_signal.update_mut(|timestamp| timestamp.recompute(&timer_mode))
        })
    }
}

/// Represents a printable timestamp.
///
/// The string representation is computed to
/// - an intuitive representation of some time ago for recent timestamps
/// - a formal timstamp for older timestamps
#[derive(Clone)]
pub struct Timestamp<TZ: TimeZone> {
    /// The display value of the timestamp.
    display: Arc<str>,

    inner: Ptr<TimestampInner<TZ>>,
}

#[derive(Clone)]
struct TimestampInner<TZ: TimeZone> {
    /// A signal that indicates how the timestamp should be printed.
    /// As the timestamp becomes older, the [TimerMode] will change.
    timer_mode_signal: XSignal<TimerMode>,

    /// Holds a reference to the closure that reacts to timer mode changes.
    timer_mode_consumers: Ptr<Option<Consumers>>,

    /// Holds a reference to the closure that reacts to timer ticks.
    /// The timer depends on the timer mode.
    timer_consumers: Ptr<Option<Consumers>>,

    /// The formal value of the timestamp.
    value: DateTime<TZ>,
}

impl<TZ: TimeZone> Timestamp<TZ> {
    #[allow(unused)]
    pub fn value(&self) -> DateTime<TZ> {
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
        let now = timer_mode
            .now()
            .map(|now| now.with_timezone(&timestamp.timezone()));

        if let Some(now) = &now {
            {
                let ago = now.clone() - timestamp.clone();
                if ago < chrono::Duration::seconds(15) {
                    self.inner.timer_mode_signal.set(TimerMode::fractions_ago());
                    return print_fractions_ago(ago);
                }
                if ago <= chrono::Duration::minutes(5) {
                    self.inner.timer_mode_signal.set(TimerMode::moments_ago());
                    return print_ago(ago);
                }
                if ago <= chrono::Duration::minutes(60) {
                    self.inner.timer_mode_signal.set(TimerMode::minutes_ago());
                    return print_ago(ago);
                }
            }

            if let (Some(now_start_of_day), Some(timestamp_start_of_day)) =
                (from_ymd_opt(now), from_ymd_opt(timestamp))
            {
                if timestamp_start_of_day == now_start_of_day {
                    self.inner.timer_mode_signal.set(TimerMode::days_ago());
                    return format!("Today, {}", hour_minute(timestamp));
                }
                if Some(timestamp_start_of_day) == now_start_of_day.checked_sub_days(Days::new(1)) {
                    self.inner.timer_mode_signal.set(TimerMode::days_ago());
                    return format!("Yesterday, {}", hour_minute(timestamp));
                }
            }
        }

        self.inner.timer_mode_signal.set(TimerMode::AbsoluteTime);
        return format!("{}, {}", day_month_year(timestamp), hour_minute(timestamp));
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
            .field("display", &self.display)
            .field("timer_mode", &self.inner.timer_mode_consumers.is_some())
            .field("timer", &self.inner.timer_consumers.is_some())
            .field("value", &self.inner.value)
            .finish()
    }
}

impl<TZ: TimeZone> PartialEq for Timestamp<TZ> {
    fn eq(&self, other: &Self) -> bool {
        self.display == other.display
            && self.inner.value.timestamp_millis() == other.inner.value.timestamp_millis()
    }
}

impl<TZ: TimeZone> Eq for Timestamp<TZ> {}

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

    fn now(&self) -> Option<DateTime<Utc>> {
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

fn print_fractions_ago(ago: chrono::Duration) -> String {
    let seconds = ago.num_seconds();
    let millis = ago.subsec_millis();
    return format!("{:0>2}.{:0>3}s ago", seconds, millis);
}

fn print_ago(mut ago: chrono::Duration) -> String {
    let hours = ago.num_hours();
    ago -= chrono::Duration::hours(hours);
    let minutes = ago.num_minutes();
    ago -= chrono::Duration::minutes(minutes);
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
