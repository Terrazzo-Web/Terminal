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
use scopeguard::defer;
use tokio::pin;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::Instrument;
use tracing::Level;
use tracing::span_enabled;
use tracing::trace;
use tracing::trace_span;
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
        let task = start_worker(state.clone(), stream, scrollback);
        let worker = if span_enabled!(Level::TRACE) {
            tokio::spawn(task.instrument(trace_span!("Worker")))
        } else {
            tokio::spawn(task)
        };
        TailStream {
            state,
            worker_handle: AbortOnDrop(worker),
        }
    }

    pub fn rewind(&mut self) {
        let mut lock = self.state.lock().unwrap();
        lock.pos = 0;
        lock.pending = None;
    }
}

/// Starts the worker that keeps reading and buffering elements.
///
/// When the buffer is full, old elements are discarded.
async fn start_worker<S>(state: Arc<Mutex<BufferState>>, stream: S, scrollback: usize)
where
    S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
{
    trace!("Start");
    defer!(trace!("Stop"));
    pin!(stream);
    let mut size = 0;
    loop {
        let item = stream.next().await;
        trace!("Next: {item:?}");
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

            let item_len = if let Some(Ok(bytes)) = &item {
                bytes.len()
            } else {
                0
            };

            // There is an off-by-one acceptable issue that the last 'None' element counts as one.
            while size + item_len > scrollback {
                trace!("size:{size} > scrollback:{scrollback}");
                // [ C1 = new oldest, C2, ... Cp, ..., C(n-1) ]
                // --> Cp becomes the (p-1) element
                let oldest = lines.drain(..1).next().unwrap();
                if *pos > 0 {
                    *pos -= 1;
                    trace! { pos, "'pos' decremented" }
                } else {
                    trace! { pos, "Buffer full, the oldest item was dropped (item={oldest:?})" }
                }
                if let Some(Ok(bytes)) = oldest {
                    size -= bytes.len();
                }
            }

            // item becomes Cb
            // [ C1 = new oldest, C2, ... Cp, ..., C(n-1), Cn = item ]
            if let Some(Ok(bytes)) = &item {
                size += bytes.len();
            }
            lines.push_back(item);

            trace!("size:{size} <= scrollback:{scrollback} lines={lines:?}");
            debug_assert!(size <= scrollback);

            if let Some(PendingBufferState { worker, .. }) = pending {
                trace! { "The stream is waiting for the next item" };
                let Some((future_tx, signal_rx2)) = worker.take() else {
                    warn! { "The {} is not waiting on the worker to produce the next item", TailStream::type_name() };
                    return;
                };
                trace! { "The stream is waking up" };
                let Ok(()) = future_tx.send(()) else {
                    warn! { "The {}'s future_rx was dropped", TailStream::type_name() };
                    return;
                };
                signal_rx = signal_rx2;
            } else {
                trace! { "The stream was not waiting on the worker to produce some data" };
                if end {
                    break; // i.e. return
                } else {
                    continue;
                }
            }
        }

        // Await after releasing the lock.
        trace! { "Wait for the stream to be woken up" };
        let _ = signal_rx.await;
        trace! { "The stream was woken up" };
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
        pos,
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
                trace! { "The stream is waking up" };
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
                trace! { "Continue waiting" };
                Some(Poll::Pending)
            }
        }
    } else if *pos < lines.len() {
        trace! { "Drain the first element, which is not a 'None'" };
        let item = lines[*pos].take();
        if let Some(Ok(bytes)) = &item {
            lines[*pos] = Some(Ok(bytes.clone()));
        }
        *pos += 1;
        Some(Poll::Ready(item))
    } else {
        trace! { "Starting to wait" };
        assert_eq!(*pos, lines.len());
        let (future_tx, future_rx) = oneshot::channel();
        let (signal_tx, signal_rx) = oneshot::channel();
        state.pending = Some(PendingBufferState {
            future_rx,
            signal_tx: Some(signal_tx),
            worker: Some((future_tx, signal_rx)),
        });
        None
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
        trace!("Aborting the worker");
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;
    use std::time::Duration;

    use bytes::Bytes;
    use futures::StreamExt;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tracing::Instrument;
    use tracing::info_span;
    use tracing::trace;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use crate::tail::TailStream;

    const TIMEOUT: Duration = Duration::from_millis(100);

    #[tokio::test]
    async fn filled() {
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
        let tail_stream = TailStream::new(stream, 12);
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

    #[tokio::test]
    async fn pending() {
        enable_tracing_for_tests();
        async {
            let (tx, rx) = mpsc::unbounded_channel();
            let stream = UnboundedReceiverStream::new(rx)
                .map(|i: i32| Ok(Bytes::from(i.to_string().into_bytes())));

            trace!("Create TailStream");
            let mut tail_stream = TailStream::new(stream, 3);
            tokio::task::yield_now().await;

            trace!("Check TailStream is empty");
            assert!(tail_stream.data(1).await.is_empty());

            trace!("Send 1 single item");
            let () = tx.send(1).unwrap();

            trace!("Read the single item");
            assert_eq!(vec!["1"], tail_stream.data(1).await);
            tokio::time::sleep(TIMEOUT).await;

            trace!("Send 10 items");
            for i in 2..10 {
                let () = tx.send(i).unwrap();
            }

            tokio::time::sleep(TIMEOUT).await;

            trace!("Read the last 3 items");
            assert_eq!(vec!["7", "8", "9"], tail_stream.data(3).await);
            trace!("Read the last 3 items -- noop already read");
            assert_eq!(Vec::<String>::default(), tail_stream.data(3).await);

            assert_eq!(tail_stream.pos(), 3);
            tail_stream.rewind();
            assert_eq!(tail_stream.pos(), 0);
            assert_eq!(vec!["7", "8", "9"], tail_stream.data(3).await);
        }
        .instrument(info_span!("Test"))
        .await
    }

    impl TailStream {
        fn data(&mut self, n: usize) -> impl Future<Output = Vec<String>> {
            self.take(n)
                .take_until(tokio::time::sleep(TIMEOUT))
                .map(|item| match item {
                    Ok(data) => String::from_utf8(Vec::from(data)).unwrap(),
                    Err(error) => error.to_string(),
                })
                .collect::<Vec<_>>()
                .instrument(info_span!("Data"))
        }

        fn pos(&self) -> usize {
            self.state.lock().unwrap().pos
        }
    }
}
