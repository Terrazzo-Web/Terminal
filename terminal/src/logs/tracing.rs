#![cfg(feature = "server")]

use std::fmt;
use std::panic::Location;

use nameth::NamedEnumValues as _;
use nameth::nameth;
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

use super::event::LogLevel;
use super::state::LogState;

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

pub struct LogStreamLayer;

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
        LogState::get().publish(level, message);
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum EnableTracingError {
    #[error("[{n}] {0}", n = self.name())]
    TryInit(#[from] TryInitError),
}
