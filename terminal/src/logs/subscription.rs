#![cfg(feature = "server")]

use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::mpsc;

use super::event::LogEvent;
use crate::logs::state::LogState;

pub struct LogSubscription {
    pub backlog: VecDeque<Arc<LogEvent>>,
    pub receiver: mpsc::UnboundedReceiver<Arc<LogEvent>>,
    subscriber_id: u64,
}

impl LogSubscription {
    pub fn new(
        subscriber_id: u64,
        backlog: VecDeque<Arc<LogEvent>>,
    ) -> (mpsc::UnboundedSender<Arc<LogEvent>>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            tx,
            Self {
                backlog,
                receiver: rx,
                subscriber_id,
            },
        )
    }
}

impl Drop for LogSubscription {
    fn drop(&mut self) {
        let _ = LogState::get().unsubscribe(self.subscriber_id);
    }
}
