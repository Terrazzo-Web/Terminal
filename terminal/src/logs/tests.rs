#![cfg(test)]

use std::sync::Mutex;

use futures::FutureExt as _;
use tokio::time::Duration;
use tracing::debug;
use tracing::dispatcher::Dispatch;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt as _;

use crate::logs::state::LogState;

use super::tracing::LogStreamLayer;

pub struct TestGuard<'t>(std::sync::MutexGuard<'t, ()>);

impl TestGuard<'_> {
    pub fn get() -> Self {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let lock = TEST_LOCK.lock().expect("test lock");
        LogState::get().reset_for_tests();
        Self(lock)
    }

    pub fn with_test_subscriber(&self, f: impl FnOnce()) {
        let subscriber = Registry::default().with(LogStreamLayer);
        tracing::dispatcher::with_default(&Dispatch::new(subscriber), f);
    }
}

#[test]
fn captures_info_warn_and_error_only() {
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
    assert_eq!(messages, vec!["info", "warn", "error"]);
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
    assert_eq!(
        subscription.backlog.front().expect("first").message,
        "event 6"
    );
    assert_eq!(
        subscription.backlog.back().expect("last").message,
        "event 25"
    );
}

#[tokio::test]
async fn replays_backlog_before_live_events() {
    let guard = TestGuard::get();
    guard.with_test_subscriber(|| {
        info!("before subscribe");
    });

    let mut subscription = LogState::get().subscribe();
    assert_eq!(subscription.backlog.len(), 1);
    assert_eq!(
        subscription.backlog.front().expect("backlog").message,
        "before subscribe"
    );

    guard.with_test_subscriber(|| {
        info!("after subscribe");
    });

    let live = tokio::time::timeout(Duration::from_secs(1), subscription.receiver.recv())
        .map(|result| result.expect("timeout").expect("event"))
        .await;
    assert_eq!(live.message, "after subscribe");
}
