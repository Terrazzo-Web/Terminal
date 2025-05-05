use std::collections::VecDeque;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;

use bytes::Bytes;
use futures::FutureExt as _;
use futures::Stream;
use futures::StreamExt as _;
use nameth::NamedType as _;
use nameth::nameth;
use tokio::pin;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::trace;
use tracing::warn;

/// A stream that only remembers the last few elements.
#[nameth]
pub struct TailStream {
    state: Arc<Mutex<BufferState>>,
    #[expect(unused)]
    worker_handle: AbortOnDrop<()>,
}

impl TailStream {
    pub fn new<S>(stream: S, scrollback: usize) -> Self
    where
        S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(BufferState::default()));
        let worker = tokio::spawn(start_worker(state.clone(), stream, scrollback));
        TailStream {
            state,
            worker_handle: AbortOnDrop(worker),
        }
    }
}

/// Starts the worker that keeps reading and buffering elements.
///
/// When the buffer is full, old elements are discarded.
async fn start_worker<S>(state: Arc<Mutex<BufferState>>, stream: S, scrollback: usize)
where
    S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
{
    pin!(stream);
    loop {
        let item = stream.next().await;
        let end = item.is_none();
        let signal_rx;
        {
            let mut lock = state.lock().expect("state");

            // [ C0 = oldest, C1, C2, ... Cp, ..., C(n-1) ]
            let BufferState {
                lines,
                pos,
                pending,
            } = &mut *lock;

            // There is an off-by-one acceptable issue that the last 'None' element counts as one.
            if lines.len() == scrollback {
                // [ C1 = new oldest, C2, ... Cp, ..., C(n-1) ]
                // --> Cp becomes the (p-1) element
                lines.drain(..1);
                if *pos > 0 {
                    *pos -= 1;
                } else {
                    // pos was already 0, the item was lost.
                }
            }

            // item becomes Cb
            // [ C1 = new oldest, C2, ... Cp, ..., C(n-1), Cn = item ]
            lines.push_back(item);

            if let Some(PendingBufferState { worker, .. }) = pending {
                // The stream is waiting for the next item.
                let Some((future_tx, signal_rx2)) = worker.take() else {
                    warn! { "The {} is not waiting on the worker to produce the next item", TailStream::type_name() };
                    return;
                };
                let Ok(()) = future_tx.send(()) else {
                    warn! { "The {}'s future_rx was dropped", TailStream::type_name() };
                    return;
                };
                signal_rx = signal_rx2;
            } else {
                continue;
            }
        }

        // Await after releasing the lock.
        let _ = signal_rx.await;
        // Now lock.pending should be reset to None.

        if end {
            // return
            break;
        }
    }
}

impl Stream for TailStream {
    type Item = std::io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.lock().expect("state");
        loop {
            let state = &mut *state;
            let result = process_state(cx, state);
            if let Some(result) = result {
                return result;
            }
        }
    }
}

fn process_state(
    cx: &mut Context<'_>,
    state: &mut BufferState,
) -> Option<Poll<Option<std::io::Result<Bytes>>>> {
    let BufferState {
        lines,
        pos: _,
        pending,
    } = state;
    if let Some(PendingBufferState {
        future_rx,
        signal_tx,
        worker,
    }) = pending.as_mut()
    {
        // Don't read from lines yet, wait for the worker to produce elements.
        match future_rx.poll_unpin(cx) {
            Poll::Ready(Ok(())) => {
                // An item has just been added to lines.
                assert!(worker.is_none());
                let result = if let Some(signal_tx) = signal_tx.take() {
                    if signal_tx.send(()).is_err() {
                        warn! { "The worker has stopped without waiting for signal_tx" };
                    }
                    None
                } else {
                    warn! { "The worker is not expecting the {} to continue", TailStream::type_name() };
                    Some(Poll::Ready(Some(Err(ErrorKind::BrokenPipe.into()))))
                };
                *pending = None;
                result
            }
            Poll::Ready(Err(oneshot::error::RecvError { .. })) => {
                warn! { "The worker has stopped without returning a new item" };
                *pending = None;
                Some(Poll::Ready(Some(Err(ErrorKind::BrokenPipe.into()))))
            }
            Poll::Pending => {
                trace!("Continue waiting");
                Some(Poll::Pending)
            }
        }
    } else {
        if lines.is_empty() {
            let (future_tx, future_rx) = oneshot::channel();
            let (signal_tx, signal_rx) = oneshot::channel();
            state.pending = Some(PendingBufferState {
                future_rx,
                signal_tx: Some(signal_tx),
                worker: Some((future_tx, signal_rx)),
            });
            None
        } else {
            // Drain the first element, which is not a 'None'.
            let item = lines.drain(..1).next().unwrap();
            Some(Poll::Ready(item))
        }
    }
}

#[derive(Default)]
struct BufferState {
    lines: VecDeque<Option<std::io::Result<Bytes>>>,

    /// The lines that have already been read.
    pos: usize,

    /// Waiting for some lines to be read
    pending: Option<PendingBufferState>,
}

struct PendingBufferState {
    /// Signal when the worker has added an item to the list.
    future_rx: oneshot::Receiver<()>,

    /// Signal that the stream has consumed the item
    /// and that the worker may continue.
    signal_tx: Option<oneshot::Sender<()>>,

    /// State of the worker to send the pending buffer and wait for the stream to consume it.
    worker: Option<(oneshot::Sender<()>, oneshot::Receiver<()>)>,
}

struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;

    use bytes::Bytes;
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use crate::tail::TailStream;

    #[tokio::test]
    async fn tail_stream() {
        enable_tracing_for_tests();
        let (tx, rx) = mpsc::unbounded_channel();
        const END: i32 = 1000;
        for i in 1..=END {
            let () = tx.send(i).unwrap();
        }
        let (end_tx, end_rx) = oneshot::channel();
        let mut end_tx = Some(end_tx);
        let stream = UnboundedReceiverStream::new(rx);
        let stream = stream
            .take_while(move |i| {
                let end = *i == END;
                if end {
                    let _ = end_tx.take().unwrap().send(());
                }
                ready(!end)
            })
            .map(|i| Ok(Bytes::from(i.to_string().into_bytes())));
        let tail_stream = TailStream::new(stream, 5);
        let _ = end_rx.await;
        let data = tail_stream
            .take(10)
            .map(|item| match item {
                Ok(data) => String::from_utf8(Vec::from(data)).unwrap(),
                Err(error) => error.to_string(),
            })
            .collect::<Vec<_>>()
            .await;
        assert_eq!(vec!["996", "997", "998", "999"], data);
    }
}
