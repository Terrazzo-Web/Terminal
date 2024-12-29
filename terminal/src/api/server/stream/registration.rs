use std::sync::Mutex;

use futures::channel::mpsc;
use terrazzo_pty::lease::ProcessOutputLease;
use tracing::debug;

use crate::api::server::correlation_id::CorrelationId;
use crate::terminal_id::TerminalId;

type OutputStreamBase = ProcessOutputLease;

#[cfg(debug_assertions)]
type OutputStream = tracing_futures::Instrumented<OutputStreamBase>;

#[cfg(not(debug_assertions))]
type OutputStream = OutputStreamBase;

pub struct Registration {
    correlation_id: CorrelationId,
    tx: mpsc::Sender<(TerminalId, OutputStream)>,
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

    pub fn get_if(correlation_id: &CorrelationId) -> Option<Registration> {
        let mut registration_lock = REGISTRATION.lock().unwrap();
        if let Some(registration) = &*registration_lock {
            if registration.correlation_id == *correlation_id {
                return registration_lock.take();
            }
        }
        return None;
    }

    pub fn set(correlation_id: CorrelationId, tx: mpsc::Sender<(TerminalId, OutputStream)>) {
        if let Some(old_registration) = std::mem::replace(
            &mut *REGISTRATION.lock().unwrap(),
            Some(Registration { correlation_id, tx }),
        ) {
            drop(old_registration);
            debug!("Removed previous registration");
        }
    }
}
