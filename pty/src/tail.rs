use std::collections::VecDeque;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::task::Context;
use std::task::Poll;

use bytes::Bytes;
use futures::FutureExt as _;
use futures::Stream;
use futures::StreamExt as _;
use futures::future::Shared;
use nameth::NamedType as _;
use nameth::nameth;
use scopeguard::defer;
use tokio::pin;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::Instrument as _;
use tracing::Level;
use tracing::span_enabled;
use tracing::trace;
use tracing::trace_span;
use tracing::warn;

/// A stream that only remembers the last few elements.
#[nameth]
#[derive(Clone)]
pub struct TailStream {
    buffer_state: Arc<Mutex<BufferState>>,
    stream_state: StreamState,
}

impl TailStream {
    pub fn new<S>(stream: S, scrollback: usize) -> Self
    where
        S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(BufferState::default()));
        let task = worker(Arc::downgrade(&state), stream, scrollback);
        let worker = if span_enabled!(Level::TRACE) {
            tokio::spawn(task.instrument(trace_span!("Worker")))
        } else {
            tokio::spawn(task)
        };
        TailStream {
            buffer_state: state,
            stream_state: StreamState {
                pos: 0,
                future_rx: None,
                worker_handle: AbortOnDrop(worker).into(),
            },
        }
    }

    pub fn rewind(&mut self) {
        self.stream_state.pos = 0;
        self.stream_state.future_rx = None;
    }
}

/// Runs the worker that keeps reading and buffering elements.
///
/// When the buffer is full, old elements are discarded.
async fn worker<S>(state: Weak<Mutex<BufferState>>, stream: S, scrollback: usize)
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
        let Some(state) = state.upgrade() else {
            trace!("All the readers have dropped");
            return;
        };
        let mut lock = state.lock().expect("state");

        // [ C0 = oldest, C1, C2, ... Cp, ..., C(n-1) ]
        let BufferState {
            lines,
            start,
            pending,
            ..
        } = &mut *lock;

        let item_len = if let Some(Ok(bytes)) = &item {
            bytes.len()
        } else {
            0
        };

        while size + item_len > scrollback {
            trace!("size:{size} > scrollback:{scrollback}");
            // [ C1 = new oldest, C2, ... Cp, ..., C(n-1) ]
            // --> Cp becomes the (p-1) element
            let oldest = lines.drain(..1).next().unwrap();
            if let Some(Ok(bytes)) = oldest {
                *start += 1;
                trace! { start, "Buffer full, the oldest item was dropped (item={bytes:?})" }
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
            if let Some(future_tx) = worker.take() {
                trace! { "The stream is waking up" };
                let Ok(()) = future_tx.send(()) else {
                    warn! { "The {}'s future_rx was dropped", TailStream::type_name() };
                    return;
                };
            }
        } else {
            trace! { "The stream was not waiting on the worker to produce some data" };
            if end {
                break; // i.e. return
            } else {
                continue;
            }
        }

        if end {
            // return
            break;
        }
    }
}

impl Stream for TailStream {
    // TODO: use Result<Bytes, Arc<std::io::Error>>
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let state = &self.buffer_state.clone();
        let mut state = state.lock().expect("state");
        let state = &mut *state;
        loop {
            let result = process_state(cx, state, &mut self.stream_state);
            if let Some(result) = result {
                return result;
            }
        }
    }
}

fn process_state(
    cx: &mut Context<'_>,
    buffer_state: &mut BufferState,
    stream_state: &mut StreamState,
) -> Option<Poll<Option<std::io::Result<Bytes>>>> {
    if stream_state.future_rx.is_some() {
        return handle_pending_state(cx, buffer_state, stream_state);
    }

    if stream_state.pos < buffer_state.start {
        trace!("Skipped {} lines", buffer_state.start - stream_state.pos);
        stream_state.pos = buffer_state.start;
    }

    if stream_state.pos - buffer_state.start < buffer_state.lines.len() {
        trace! { "Drain the first element, which is not a 'None'" };
        let item = buffer_state.lines[stream_state.pos - buffer_state.start].take();
        if let Some(item) = &item {
            buffer_state.lines[stream_state.pos - buffer_state.start] = match &item {
                Ok(bytes) => Some(Ok(bytes.clone())),
                Err(error) => Some(Err(std::io::Error::new(error.kind(), error.to_string()))),
            }
        }
        stream_state.pos += 1;
        return Some(Poll::Ready(item));
    }

    {
        trace! { "Starting to wait" };
        assert_eq!(
            stream_state.pos - buffer_state.start,
            buffer_state.lines.len()
        );
        if let Some(pending_buffer_state) = &mut buffer_state.pending {
            stream_state.future_rx = Some(pending_buffer_state.future_rx.clone());
        } else {
            let (future_tx, future_rx) = oneshot::channel();
            let future_rx = future_rx.shared();
            stream_state.future_rx = Some(future_rx.clone());
            buffer_state.pending = Some(PendingBufferState {
                future_rx,
                worker: Some(future_tx),
            });
        }
        return None;
    }
}

/// Handles the case when streams are waiting on the worker to produce some data.
fn handle_pending_state(
    cx: &mut Context<'_>,
    buffer_state: &mut BufferState,
    stream_state: &mut StreamState,
) -> Option<Poll<Option<Result<Bytes, std::io::Error>>>> {
    let future_rx = if let Some(future_rx) = &mut stream_state.future_rx {
        future_rx
    } else {
        let pending_buffer_state = buffer_state.pending.as_mut()?;
        stream_state.future_rx = pending_buffer_state.future_rx.clone().into();
        unsafe { stream_state.future_rx.as_mut().unwrap_unchecked() }
    };
    match future_rx.poll_unpin(cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Ready(Err(oneshot::error::RecvError { .. })) => {
            warn! { "The worker has stopped without returning a new item" };
            buffer_state.pending = None;
            return Some(Poll::Ready(Some(Err(ErrorKind::BrokenPipe.into()))));
        }
        Poll::Pending => {
            trace! { "Continue waiting" };
            return Some(Poll::Pending);
        }
    }

    stream_state.future_rx = None;
    let pending_buffer_state = buffer_state.pending.take()?;

    trace! { "The stream is waking up" };
    // An item has just been added to lines.
    assert!(pending_buffer_state.worker.is_none());
    return None;
}

#[derive(Default)]
struct BufferState {
    lines: VecDeque<Option<std::io::Result<Bytes>>>,

    /// The line number at which 'lines' starts, ie, the number of lines that were discarded.
    start: usize,

    /// Waiting for some lines to be read
    pending: Option<PendingBufferState>,
}

struct StreamState {
    pos: usize,
    future_rx: Option<Shared<oneshot::Receiver<()>>>,
    worker_handle: Arc<AbortOnDrop<()>>,
}

impl Clone for StreamState {
    fn clone(&self) -> Self {
        Self {
            pos: 0,
            future_rx: None,
            worker_handle: self.worker_handle.clone(),
        }
    }
}

struct PendingBufferState {
    /// Signal when the worker has added an item to the list.
    future_rx: Shared<oneshot::Receiver<()>>,

    /// State of the worker to send the pending buffer.
    worker: Option<oneshot::Sender<()>>,
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
    use futures::StreamExt as _;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tracing::Instrument as _;
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
            trace!("Read the last 3 items -- after rewind");
            assert_eq!(vec!["7", "8", "9"], tail_stream.data(3).await);
        }
        .instrument(info_span!("Test"))
        .await
    }

    #[tokio::test]
    async fn shared() {
        enable_tracing_for_tests();
        async {
            let (tx, rx) = mpsc::unbounded_channel();
            let stream = UnboundedReceiverStream::new(rx)
                .map(|i: i32| Ok(Bytes::from(i.to_string().into_bytes())));

            trace!("Create TailStream");
            let mut tail_stream1 = TailStream::new(stream, 3);
            let mut tail_stream2 = tail_stream1.clone();
            tokio::task::yield_now().await;

            trace!("Check TailStream is empty");
            assert!(tail_stream1.data(1).await.is_empty());
            assert!(tail_stream2.data(1).await.is_empty());

            trace!("Send 1 single item");
            let () = tx.send(1).unwrap();

            trace!("Read the single item");
            assert_eq!(vec!["1"], tail_stream1.data(1).await);
            assert_eq!(vec!["1"], tail_stream2.data(1).await);
            tokio::time::sleep(TIMEOUT).await;

            trace!("Send 10 items");
            for i in 2..10 {
                let () = tx.send(i).unwrap();
            }

            tokio::time::sleep(TIMEOUT).await;

            trace!("Read the last 3 items");
            assert_eq!(vec!["7", "8", "9"], tail_stream1.data(3).await);
            assert_eq!(vec!["7", "8", "9"], tail_stream2.data(3).await);
            trace!("Read the last 3 items -- noop already read");
            assert_eq!(Vec::<String>::default(), tail_stream1.data(3).await);
            assert_eq!(Vec::<String>::default(), tail_stream2.data(3).await);

            assert_eq!(tail_stream1.pos(), 3);
            assert_eq!(tail_stream2.pos(), 3);
            tail_stream1.rewind();
            assert_eq!(tail_stream1.pos(), 0);
            trace!("Read the last 3 items -- after rewind");
            assert_eq!(vec!["7", "8", "9"], tail_stream1.data(3).await);

            let mut tail_stream3 = tail_stream1.clone();
            assert_eq!(vec!["7", "8", "9"], tail_stream3.data(3).await);
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
            self.stream_state
                .pos
                .saturating_sub(self.buffer_state.lock().unwrap().start)
        }
    }
}
