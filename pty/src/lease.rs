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
use tracing::debug;
use tracing::error;
use tracing::info;

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
        info!("Drop {}", ProcessIoEntry::type_name());
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
        debug!("Stop current lease");
        let _ = self.signal_tx.send(());
        let process_output = self.process_output_rx.await?;
        debug!("Got new lease");
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

#[derive(Default)]
pub struct ProcessOutputLease {
    maybe_output: Option<TakeUntil<ReleaseOnDrop<ProcessOutput>, oneshot::Receiver<()>>>,
}

impl ProcessOutputLease {
    fn new(
        process_output: ProcessOutput,
    ) -> (Self, oneshot::Sender<()>, oneshot::Receiver<ProcessOutput>) {
        let (process_output, process_output_rx) = ReleaseOnDrop::new(process_output);
        let (signal_tx, signal_rx) = oneshot::channel();
        let lease = Self {
            maybe_output: Some(process_output.take_until(signal_rx)),
        };
        (lease, signal_tx, process_output_rx)
    }

    fn release(&mut self) {
        *self = Self::default()
    }
}

impl Stream for ProcessOutputLease {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let next = {
            let Some(process_io) = self.maybe_output.as_mut() else {
                // Normally this can't happen.
                // Another terminal can "steal" the process using signaling,
                // which closes and releases this stream.
                error!("The process output lease was lost while streaming");
                return None.into();
            };
            ready!(process_io.poll_next_unpin(cx))
        };

        if next.is_some() {
            match &next {
                Some(Ok(data)) => {
                    debug!("Reading {}", String::from_utf8_lossy(data).escape_default())
                }
                Some(Err(error)) => error!("Reading failed: {error}"),
                None => (),
            }
            return next.into();
        }

        self.release();
        return None.into();
    }
}

impl Stream for ReleaseOnDrop<ProcessOutput> {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next_unpin(cx)
    }
}
