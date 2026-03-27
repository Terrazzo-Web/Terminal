#![cfg(test)]
#![cfg(feature = "server")]

use std::sync::Mutex;

use futures::FutureExt as _;
use tokio::time::Duration;
use tracing::debug;
use tracing::dispatcher::Dispatch;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt as _;

use super::tracing::LogStreamLayer;
use crate::logs::state::LogState;

pub struct TestGuard<'t>(std::sync::MutexGuard<'t, ()>);

impl TestGuard<'_> {
    pub fn get() -> Self {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let lock = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        LogState::get().reset_for_tests();
        Self(lock)
    }

    pub fn with_test_subscriber(&self, f: impl FnOnce()) {
        let subscriber = Registry::default().with(LogStreamLayer);
        tracing::dispatcher::with_default(&Dispatch::new(subscriber), f);
    }
}

#[test]
fn captures_debug_info_warn_and_error() {
    let guard = TestGuard::get();
    guard.with_test_subscriber(|| {
        debug!("debug");
        info!("info");
        warn!("warn");
        error!("error");
    });

    let mut subscription = LogState::get().subscribe();
    let messages: Vec<_> = std::mem::take(&mut subscription.backlog)
        .into_iter()
        .map(|log| log.message.clone())
        .collect();
    assert_eq!(messages.len(), 4);
    assert!(messages[0] == "debug", "Got {:?}", messages[0]);
    assert!(messages[1] == "info", "Got {:?}", messages[1]);
    assert!(messages[2] == "warn", "Got {:?}", messages[2]);
    assert!(messages[3] == "error", "Got {:?}", messages[3]);
}

#[test]
fn includes_span_context_and_source_location() {
    let guard = TestGuard::get();
    guard.with_test_subscriber(|| {
        let span = info_span!("Polling config file", config_file_path = "/tmp/config.toml");
        let _entered = span.enter();
        debug!("Polling config file");
    });

    let subscription = LogState::get().subscribe();
    let message = &subscription.backlog.front().expect("log").message;
    assert!(message.contains("Polling config file: "));
    assert!(message.contains("terminal/src/logs/tests.rs:"));
    assert!(message.contains("Polling config file"));
    assert!(message.contains("config_file_path=\"/tmp/config.toml\""));
}

#[test]
fn keeps_only_the_newest_twenty_logs() {
    let guard = TestGuard::get();
    guard.with_test_subscriber(|| {
        for index in 0..=25 {
            info!("event {index}");
        }
    });

    let subscription = LogState::get().subscribe();
    assert_eq!(subscription.backlog.len(), 20);
    let first = subscription.backlog.front().expect("first");
    assert!(first.message.ends_with(": event 6"), "{first:?}");
    let last = subscription.backlog.back().expect("last");
    assert!(last.message.ends_with(": event 25"), "{last:?}");
}

#[tokio::test]
async fn replays_backlog_before_live_events() {
    let guard = TestGuard::get();
    guard.with_test_subscriber(|| {
        info!("before subscribe");
    });

    let mut subscription = LogState::get().subscribe();
    assert_eq!(subscription.backlog.len(), 1);
    assert!(
        subscription
            .backlog
            .front()
            .expect("backlog")
            .message
            .ends_with(": before subscribe")
    );

    guard.with_test_subscriber(|| {
        info!("after subscribe");
    });

    let live = tokio::time::timeout(Duration::from_secs(1), subscription.receiver.recv())
        .map(|result| result.expect("timeout").expect("event"))
        .await;
    assert!(live.message.ends_with(": after subscribe"));
}
