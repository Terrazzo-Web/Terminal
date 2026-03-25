#![cfg(feature = "server")]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::panic::Location;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::sync::mpsc;
use tracing::Event;
use tracing::Subscriber;
use tracing::field::Field;
use tracing::field::Visit;
use tracing::level_filters::LevelFilter;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::util::TryInitError;

const BACKLOG_CAPACITY: usize = 20;

pub fn init_tracing() -> Result<(), EnableTracingError> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions))
        .with_target(false);

    tracing_subscriber::registry()
        .with(
            EnvFilter::new("debug,tower=info,h2=info,hyper_util=info")
                .add_directive(LevelFilter::DEBUG.into()),
        )
        .with(fmt_layer)
        .with(LogStreamLayer)
        .try_init()?;

    std::panic::set_hook(Box::new(|panic_info| {
        let panic_payload: Option<&str> =
            if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                Some(s)
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                Some(s.as_str())
            } else {
                None
            };
        let location = panic_info
            .location()
            .map(Location::to_string)
            .unwrap_or_else(|| "???".into());
        if let Some(panic_payload) = panic_payload {
            warn!("Panic: {panic_payload} at {location}");
        } else {
            warn!("Panic at {location}");
        }
    }));
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UiLogEvent {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "i"))]
    pub id: u64,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "l"))]
    pub level: LogLevel,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "m"))]
    pub message: String,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "t"))]
    pub timestamp_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "I"))]
    Info,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "W"))]
    Warn,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
        .fmt(f)
    }
}

pub struct LogSubscription {
    pub backlog: Vec<UiLogEvent>,
    pub receiver: mpsc::UnboundedReceiver<UiLogEvent>,
    subscriber_id: u64,
}

impl Drop for LogSubscription {
    fn drop(&mut self) {
        let _ = log_state().remove_subscriber(self.subscriber_id);
    }
}

pub fn subscribe() -> LogSubscription {
    log_state().subscribe()
}

struct LogStreamLayer;

impl<S> Layer<S> for LogStreamLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let level = match *event.metadata().level() {
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
            _ => return,
        };

        let mut visitor = LogEventVisitor::default();
        event.record(&mut visitor);
        let message = visitor.finish();
        log_state().publish(level, message);
    }
}

#[derive(Default)]
struct LogEventVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl LogEventVisitor {
    fn record_value(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push(format!("{}={value}", field.name()));
        }
    }

    fn finish(self) -> String {
        match (self.message, self.fields.is_empty()) {
            (Some(message), true) => message,
            (Some(message), false) => format!("{message} {}", self.fields.join(" ")),
            (None, false) => self.fields.join(" "),
            (None, true) => "log event".to_owned(),
        }
    }
}

impl Visit for LogEventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, value.to_owned());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, value.to_string());
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.record_value(field, value.to_string());
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.record_value(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_value(field, value.to_string());
    }
}

fn log_state() -> &'static LogState {
    static LOG_STATE: OnceLock<LogState> = OnceLock::new();
    LOG_STATE.get_or_init(LogState::default)
}

#[derive(Default)]
struct LogState {
    next_event_id: AtomicU64,
    inner: Mutex<LogStateInner>,
}

#[derive(Default)]
struct LogStateInner {
    next_subscriber_id: u64,
    backlog: VecDeque<UiLogEvent>,
    subscribers: HashMap<u64, mpsc::UnboundedSender<UiLogEvent>>,
}

impl LogState {
    fn publish(&self, level: LogLevel, message: String) {
        let event = UiLogEvent {
            id: self.next_event_id.fetch_add(1, Ordering::Relaxed) + 1,
            level,
            message,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        let mut inner = self.inner.lock().expect("log stream");
        inner.backlog.push_back(event.clone());
        if inner.backlog.len() > BACKLOG_CAPACITY {
            let _ = inner.backlog.pop_front();
        }

        inner
            .subscribers
            .retain(|_, sender| sender.send(event.clone()).is_ok());
    }

    fn subscribe(&self) -> LogSubscription {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut inner = self.inner.lock().expect("log stream");
        let subscriber_id = inner.next_subscriber_id;
        inner.next_subscriber_id += 1;
        let backlog = inner.backlog.iter().cloned().collect();
        inner.subscribers.insert(subscriber_id, tx);
        LogSubscription {
            backlog,
            receiver: rx,
            subscriber_id,
        }
    }

    fn remove_subscriber(&self, subscriber_id: u64) -> bool {
        self.inner
            .lock()
            .expect("log stream")
            .subscribers
            .remove(&subscriber_id)
            .is_some()
    }

    #[cfg(test)]
    fn reset_for_tests(&self) {
        self.next_event_id.store(0, Ordering::Relaxed);
        *self.inner.lock().expect("log stream") = LogStateInner::default();
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum EnableTracingError {
    #[error("[{n}] {0}", n = self.name())]
    TryInit(#[from] TryInitError),
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use futures::FutureExt as _;
    use tokio::time::Duration;
    use tracing::dispatcher::Dispatch;
    use tracing_subscriber::Registry;

    use super::*;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        TEST_LOCK.lock().expect("test lock")
    }

    fn with_test_subscriber(f: impl FnOnce()) {
        let subscriber = Registry::default().with(LogStreamLayer);
        tracing::dispatcher::with_default(&Dispatch::new(subscriber), f);
    }

    #[test]
    fn captures_info_warn_and_error_only() {
        let _guard = test_lock();
        log_state().reset_for_tests();
        with_test_subscriber(|| {
            tracing::debug!("debug");
            tracing::info!("info");
            tracing::warn!("warn");
            tracing::error!("error");
        });

        let subscription = subscribe();
        let messages: Vec<_> = subscription
            .backlog
            .into_iter()
            .map(|log| log.message)
            .collect();
        assert_eq!(messages, vec!["info", "warn", "error"]);
    }

    #[test]
    fn keeps_only_the_newest_twenty_logs() {
        let _guard = test_lock();
        log_state().reset_for_tests();
        with_test_subscriber(|| {
            for index in 0..21 {
                tracing::info!("event {index}");
            }
        });

        let subscription = subscribe();
        assert_eq!(subscription.backlog.len(), 20);
        assert_eq!(
            subscription.backlog.first().expect("first").message,
            "event 1"
        );
        assert_eq!(
            subscription.backlog.last().expect("last").message,
            "event 20"
        );
    }

    #[tokio::test]
    async fn replays_backlog_before_live_events() {
        let _guard = test_lock();
        log_state().reset_for_tests();
        with_test_subscriber(|| {
            tracing::info!("before subscribe");
        });

        let mut subscription = subscribe();
        assert_eq!(subscription.backlog.len(), 1);
        assert_eq!(subscription.backlog[0].message, "before subscribe");

        with_test_subscriber(|| {
            tracing::info!("after subscribe");
        });

        let live = tokio::time::timeout(Duration::from_secs(1), subscription.receiver.recv())
            .map(|result| result.expect("timeout").expect("event"))
            .await;
        assert_eq!(live.message, "after subscribe");
    }
}
