use std::pin::Pin;
use std::sync::Arc;
use std::task::ready;
use std::task::Poll;

use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::TakeUntil;
use futures::Stream;
use futures::StreamExt as _;
use named::named;
use named::NamedEnumValues as _;
use named::NamedType as _;
use scopeguard::defer;
use tracing::debug;
use tracing::debug_span;
use tracing::error;
use tracing::info;
use tracing::trace;

use crate::release_on_drop::ReleaseOnDrop;
use crate::ProcessIO;
use crate::ProcessInput;
use crate::ProcessOutput;

#[named]
pub struct ProcessIoEntry {
    input: Mutex<ProcessInput>,
    output: Mutex<Option<ProcessOutputExchange>>,
}

impl ProcessIoEntry {
    pub fn new(process_io: ProcessIO) -> Arc<Self> {
        info!("Create {}", ProcessIoEntry::type_name());
        let (input, output) = process_io.split();
        Arc::new(Self {
            input: Mutex::new(input),
            output: Mutex::new(Some(ProcessOutputExchange::new(output))),
        })
    }

    pub async fn lease_output(
        self: &Arc<Self>,
    ) -> Result<ProcessOutputLease, LeaseProcessOutputError> {
        let mut lock = self.output.lock().await;
        let exchange = lock.take().ok_or(LeaseProcessOutputError::OutputNotSet)?;
        let (lease, exchange) = exchange.lease().await?;
        *lock = Some(exchange);
        return Ok(lease);
    }

    pub async fn input(&self) -> futures::lock::MutexGuard<ProcessInput> {
        self.input.lock().await
    }
}

impl Drop for ProcessIoEntry {
    fn drop(&mut self) {
        info!("Drop {}", Self::type_name());
    }
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum LeaseProcessOutputError {
    #[error("[{n}] Output not set", n = self.name())]
    OutputNotSet,

    #[error("[{n}] {0}", n = self.name())]
    LeaseError(#[from] LeaseError),
}

struct ProcessOutputExchange {
    signal_tx: oneshot::Sender<()>,
    process_output_rx: oneshot::Receiver<ProcessOutput>,
}

impl ProcessOutputExchange {
    fn new(process_output: ProcessOutput) -> Self {
        let (_lease, signal_tx, process_output_rx) = ProcessOutputLease::new(process_output);
        Self {
            signal_tx,
            process_output_rx,
        }
    }

    async fn lease(self) -> Result<(ProcessOutputLease, Self), LeaseError> {
        match self.signal_tx.send(()) {
            Ok(()) => debug!("Current lease was stopped"),
            Err(()) => debug!("The process was not leased"),
        }
        debug!("Getting new lease...");
        let process_output = self.process_output_rx.await?;
        debug!("Getting new lease: Done");
        let (lease, signal_tx, process_output_rx) = ProcessOutputLease::new(process_output);
        Ok((
            lease,
            Self {
                signal_tx,
                process_output_rx,
            },
        ))
    }
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum LeaseError {
    #[error("[{n}] Canceled", n = self.name())]
    Canceled(#[from] oneshot::Canceled),
}

#[named]
pub enum ProcessOutputLease {
    /// The process is active and this is the current lease.
    Leased(TakeUntil<ReleaseOnDrop<ProcessOutput>, oneshot::Receiver<()>>),

    /// The process is still active but another client is consuming the stream.
    Revoked,

    /// The process is closed. We return one last [LeaseItem] to indicate the closure.
    Closed,
}

impl ProcessOutputLease {
    fn new(
        process_output: ProcessOutput,
    ) -> (Self, oneshot::Sender<()>, oneshot::Receiver<ProcessOutput>) {
        let (process_output, process_output_rx) = ReleaseOnDrop::new(process_output);
        let (signal_tx, signal_rx) = oneshot::channel();
        let process_output = process_output.take_until(signal_rx);
        let lease = Self::Leased(process_output);
        (lease, signal_tx, process_output_rx)
    }

    fn revoke(&mut self) {
        let _span = debug_span!("Revoking").entered();
        debug!("Start");
        defer!(debug!("End"));
        *self = Self::Revoked
    }
}

impl Stream for ProcessOutputLease {
    type Item = LeaseItem;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        trace!("Poll next: state={}", self.name());
        let next = {
            let process_io = match &mut *self {
                ProcessOutputLease::Leased(process_io) => process_io,
                ProcessOutputLease::Revoked => return None.into(),
                ProcessOutputLease::Closed => {
                    self.revoke();
                    return Some(LeaseItem::EOS).into();
                }
            };
            let next = ready!(process_io.poll_next_unpin(cx));
            if next.is_none() && process_io.is_stopped() {
                match process_io.take_result() {
                    Some(Err(oneshot::Canceled)) | None => {
                        debug!("The process ended");
                        self.revoke();
                        return Some(LeaseItem::EOS).into();
                    }
                    Some(Ok(())) => debug!("The lease was revoked"),
                }
            }
            trace! { "next.is_none={} process_io.is_stopped={}", next.is_none(), process_io.is_stopped() };
            next
        };

        Some(match next {
            Some(Ok(data)) => {
                debug_assert!(!data.is_empty(), "Unexpected empty buffer");
                debug! { "Reading {}", String::from_utf8_lossy(&data).escape_default() }
                LeaseItem::Data(data)
            }
            Some(Err(error)) => {
                trace!("Reading failed: {error}");
                LeaseItem::Error(error)
            }
            None => {
                debug!("next is None");
                self.revoke();
                return None.into();
            }
        })
        .into()
    }
}

#[named]
pub enum LeaseItem {
    EOS,
    Data(Vec<u8>),
    Error(std::io::Error),
}

impl Stream for ReleaseOnDrop<ProcessOutput> {
    type Item = <ProcessOutput as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next_unpin(cx)
    }
}
