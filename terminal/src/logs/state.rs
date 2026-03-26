#![cfg(feature = "server")]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tokio::sync::mpsc;

use super::event::LogEvent;
use super::event::LogLevel;
use super::subscription::LogSubscription;

const BACKLOG_CAPACITY: usize = 20;

#[derive(Default)]
pub struct LogState {
    next_event_id: AtomicU64,
    inner: Mutex<LogStateInner>,
}

impl LogState {
    pub fn get() -> &'static Self {
        static INSTANCE: OnceLock<LogState> = OnceLock::new();
        INSTANCE.get_or_init(|| LogState::default())
    }
}

#[derive(Default)]
struct LogStateInner {
    next_subscriber_id: u64,
    backlog: VecDeque<Arc<LogEvent>>,
    subscribers: HashMap<u64, mpsc::UnboundedSender<Arc<LogEvent>>>,
}

impl LogState {
    pub fn publish(&self, level: LogLevel, message: String) {
        let event = Arc::new(LogEvent {
            id: self.next_event_id.fetch_add(1, Ordering::Relaxed) + 1,
            level,
            message,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

        let mut inner = self.lock();
        inner
            .subscribers
            .retain(|_, sender| sender.send(event.clone()).is_ok());

        inner.backlog.push_back(event);
        if inner.backlog.len() > BACKLOG_CAPACITY {
            let _ = inner.backlog.pop_front();
        }
    }

    pub fn subscribe(&self) -> LogSubscription {
        let mut inner = self.lock();
        let subscriber_id = inner.get_next_subscriber_id();
        let backlog = inner.backlog.clone();
        let (tx, subscription) = LogSubscription::new(subscriber_id, backlog);
        inner.subscribers.insert(subscriber_id, tx);
        subscription
    }

    pub fn unsubscribe(&self, subscriber_id: u64) -> bool {
        self.lock().subscribers.remove(&subscriber_id).is_some()
    }

    #[cfg(test)]
    pub fn reset_for_tests(&self) {
        self.next_event_id.store(0, Ordering::Relaxed);
        *self.inner.lock().expect("log stream") = LogStateInner::default();
    }

    fn lock(&self) -> MutexGuard<'_, LogStateInner> {
        self.inner.lock().expect("log state")
    }
}

impl LogStateInner {
    fn get_next_subscriber_id(&mut self) -> u64 {
        let next = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        next
    }
}
