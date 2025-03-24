use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::task::ready;

use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::stream::TakeUntil;
use nameth::NamedEnumValues as _;
use nameth::NamedType as _;
use nameth::nameth;
use scopeguard::defer;
use tokio_util::io::ReaderStream;
use tracing::debug;
use tracing::debug_span;
use tracing::error;
use tracing::info;
use tracing::trace;

use crate::IsDataStream;
use crate::ProcessIO;
use crate::ProcessInput;
use crate::ProcessOutput;
use crate::pty::OwnedReadPty;
use crate::pty::OwnedWritePty;
use crate::release_on_drop::ReleaseOnDrop;

#[nameth]
pub struct ProcessIoEntry<W = OwnedWritePty, R = ReaderStream<OwnedReadPty>> {
    input: Mutex<ProcessInput<W>>,
    output: Mutex<Option<ProcessOutputExchange<R>>>,
}

impl<W, R: IsDataStream> ProcessIoEntry<W, R> {
    pub fn new(process_io: ProcessIO<W, R>) -> Arc<Self> {
        info!("Create {}", Self::type_name());
        let (input, output) = process_io.split();
        Arc::new(Self {
            input: Mutex::new(input),
            output: Mutex::new(Some(ProcessOutputExchange::new(output))),
        })
    }

    pub async fn lease_output(
        self: &Arc<Self>,
    ) -> Result<ProcessOutputLease<R>, LeaseProcessOutputError> {
        let mut lock = self.output.lock().await;
        let exchange = lock.take().ok_or(LeaseProcessOutputError::OutputNotSet)?;
        let (lease, exchange) = exchange.lease().await?;
        *lock = Some(exchange);
        return Ok(lease);
    }

    pub async fn input(&self) -> futures::lock::MutexGuard<ProcessInput<W>> {
        self.input.lock().await
    }
}

impl<W, R> Drop for ProcessIoEntry<W, R> {
    fn drop(&mut self) {
        info!("Drop {}", Self::type_name());
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum LeaseProcessOutputError {
    #[error("[{n}] Output not set", n = self.name())]
    OutputNotSet,

    #[error("[{n}] {0}", n = self.name())]
    LeaseError(#[from] LeaseError),
}

struct ProcessOutputExchange<R = ReaderStream<OwnedReadPty>> {
    signal_tx: oneshot::Sender<()>,
    process_output_rx: oneshot::Receiver<ProcessOutput<R>>,
}

impl<R: IsDataStream> ProcessOutputExchange<R> {
    fn new(process_output: ProcessOutput<R>) -> Self {
        let (_lease, signal_tx, process_output_rx) = ProcessOutputLease::new(process_output);
        Self {
            signal_tx,
            process_output_rx,
        }
    }

    async fn lease(self) -> Result<(ProcessOutputLease<R>, Self), LeaseError> {
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

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum LeaseError {
    #[error("[{n}] Canceled", n = self.name())]
    Canceled(#[from] oneshot::Canceled),
}

#[nameth]
pub enum ProcessOutputLease<R: IsDataStream = ReaderStream<OwnedReadPty>> {
    /// The process is active and this is the current lease.
    Leased(TakeUntil<ReleaseOnDrop<ProcessOutput<R>>, oneshot::Receiver<()>>),

    /// The process is still active but another client is consuming the stream.
    Revoked,

    /// The process is closed. We return one last [LeaseItem] to indicate the closure.
    Closed,
}

impl<R: IsDataStream> ProcessOutputLease<R> {
    fn new(
        process_output: ProcessOutput<R>,
    ) -> (
        Self,
        oneshot::Sender<()>,
        oneshot::Receiver<ProcessOutput<R>>,
    ) {
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

impl<R: IsDataStream> Stream for ProcessOutputLease<R> {
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

#[nameth]
pub enum LeaseItem {
    EOS,
    Data(Vec<u8>),
    Error(std::io::Error),
}

impl<R: IsDataStream> Stream for ReleaseOnDrop<ProcessOutput<R>> {
    type Item = <ProcessOutput<R> as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next_unpin(cx)
    }
}
