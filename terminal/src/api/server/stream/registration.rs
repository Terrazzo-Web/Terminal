use std::sync::Mutex;
use std::time::Duration;

use futures::channel::mpsc;
use futures::channel::oneshot;
use tracing::debug;

use crate::api::server::correlation_id::CorrelationId;
use crate::processes::io::LocalReader;
use crate::terminal_id::TerminalId;

type OutputStreamBase = LocalReader;

#[cfg(debug_assertions)]
type OutputStream = tracing_futures::Instrumented<OutputStreamBase>;

#[cfg(not(debug_assertions))]
type OutputStream = OutputStreamBase;

pub struct Registration {
    correlation_id: CorrelationId,
    tx: mpsc::Sender<(TerminalId, OutputStream)>,
    keepalive: Option<oneshot::Sender<()>>,
}

static REGISTRATION: Mutex<Option<Registration>> = Mutex::new(None);

impl Registration {
    pub fn current() -> Option<mpsc::Sender<(TerminalId, OutputStream)>> {
        REGISTRATION
            .lock()
            .unwrap()
            .as_ref()
            .map(|registration| registration.tx.clone())
    }

    pub fn take_if(correlation_id: &CorrelationId) -> Option<Registration> {
        let mut lock = REGISTRATION.lock().unwrap();
        let Some(current) = &*lock else {
            return None;
        };
        if current.correlation_id == *correlation_id {
            return lock.take();
        }
        return None;
    }

    pub fn take_keepalive(correlation_id: &CorrelationId) -> Option<oneshot::Sender<()>> {
        let mut lock = REGISTRATION.lock().unwrap();
        let Some(current) = &mut *lock else {
            return None;
        };
        if current.correlation_id == *correlation_id {
            return current.keepalive.take();
        }
        return None;
    }

    pub fn set(
        correlation_id: CorrelationId,
    ) -> (
        mpsc::Receiver<(TerminalId, OutputStream)>,
        oneshot::Receiver<()>,
    ) {
        let (tx, rx) = mpsc::channel(10);
        let (keepalive_tx, keepalive_rx) = oneshot::channel();
        if let Some(old_registration) = std::mem::replace(
            &mut *REGISTRATION.lock().unwrap(),
            Some(Registration {
                correlation_id: correlation_id.clone(),
                tx,
                keepalive: Some(keepalive_tx),
            }),
        ) {
            drop(old_registration);
            debug!("Removed previous registration");
        }
        tokio::spawn(timeout_keepalive(correlation_id));
        (rx, keepalive_rx)
    }
}

async fn timeout_keepalive(correlation_id: CorrelationId) {
    tokio::time::sleep(Duration::from_secs(5)).await;
    let mut current = REGISTRATION.lock().unwrap();
    let Some(current) = &mut *current else {
        return;
    };
    if current.correlation_id != correlation_id {
        return;
    }
    current.keepalive = None;
}
