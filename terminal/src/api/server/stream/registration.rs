use std::sync::Mutex;

use futures::channel::mpsc;
use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tokio::task::JoinHandle;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info;
use trz_gateway_common::http_error::IsHttpError;

use super::pipe::PIPE_TTL;
use crate::api::TerminalAddress;
use crate::api::server::correlation_id::CorrelationId;
use crate::processes::io::LocalReader;

type OutputStreamBase = LocalReader;

#[cfg(debug_assertions)]
type OutputStream = tracing_futures::Instrumented<OutputStreamBase>;

#[cfg(not(debug_assertions))]
type OutputStream = OutputStreamBase;

pub struct Registration {
    correlation_id: CorrelationId,
    tx: mpsc::Sender<(TerminalAddress, OutputStream)>,
    timeout_tx: Option<oneshot::Sender<()>>,
    timeout_handle: JoinHandle<()>,
}

static REGISTRATION: Mutex<Option<Registration>> = Mutex::new(None);

impl Registration {
    pub fn current() -> Option<mpsc::Sender<(TerminalAddress, OutputStream)>> {
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

    pub fn ping_timeout(correlation_id: &CorrelationId) -> Result<(), PingTimeoutError> {
        let mut lock = REGISTRATION.lock().unwrap();
        let Some(current) = &mut *lock else {
            return Err(PingTimeoutError::NoActiveRegistration);
        };
        if current.correlation_id != *correlation_id {
            return Err(PingTimeoutError::CorrelationIdMismatch);
        }
        current.timeout_handle.abort();
        current.timeout_handle = timeout_handle(correlation_id.to_owned());
        Ok(())
    }

    pub fn set(
        correlation_id: CorrelationId,
    ) -> (
        mpsc::Receiver<(TerminalAddress, OutputStream)>,
        impl Future<Output = ()>,
    ) {
        let (tx, rx) = mpsc::channel(10);
        let (timeout_tx, timeout_rx) = oneshot::channel();
        if let Some(old_registration) = (*REGISTRATION.lock().unwrap()).replace(Registration {
            correlation_id: correlation_id.clone(),
            tx,
            timeout_tx: Some(timeout_tx),
            timeout_handle: timeout_handle(correlation_id),
        }) {
            drop(old_registration);
            debug!("Removed previous registration");
        }
        let keepalive = async move {
            match timeout_rx.await {
                Ok(()) => info!("Timed out"),
                Err(oneshot::Canceled) => info!("Canceled"),
            }
        };
        (rx, keepalive)
    }
}

fn timeout_handle(correlation_id: CorrelationId) -> JoinHandle<()> {
    tokio::spawn(timeout_keepalive(correlation_id).in_current_span())
}

async fn timeout_keepalive(correlation_id: CorrelationId) {
    tokio::time::sleep(PIPE_TTL).await;
    let mut current = REGISTRATION.lock().unwrap();
    let Some(current) = &mut *current else {
        debug!("No current pipe registration");
        return;
    };
    if current.correlation_id != correlation_id {
        debug!(
            "Current pipe registration is for a different current.correlation_id:{}",
            current.correlation_id
        );
        return;
    }
    if let Some(keepalive) = current.timeout_tx.take() {
        info!("Timing out the stream");
        let _ = keepalive.send(());
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PingTimeoutError {
    #[error("[{n}] Pipe is not active", n = self.name())]
    NoActiveRegistration,

    #[error("[{n}] Correlation ID not found", n = self.name())]
    CorrelationIdMismatch,
}

impl IsHttpError for PingTimeoutError {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        match self {
            Self::NoActiveRegistration => StatusCode::NOT_FOUND,
            Self::CorrelationIdMismatch => StatusCode::NOT_FOUND,
        }
    }
}
