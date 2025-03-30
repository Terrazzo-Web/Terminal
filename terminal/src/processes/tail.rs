#![allow(unused)]

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;

use futures::FutureExt as _;
use futures::Stream;
use futures::StreamExt as _;
use nameth::NamedType as _;
use nameth::nameth;
use pin_project::pin_project;
use terrazzo::prelude::UpdateAndReturn;
use terrazzo_pty::lease::LeaseItem;
use tokio::pin;
use tokio::stream;
use tokio::sync::oneshot;
use tracing::warn;

#[pin_project]
#[nameth]
pub struct TailStream {
    state: Arc<Mutex<BufferState>>,
}

impl TailStream {
    pub fn new<S>(stream: S, scrollback: usize) -> Self
    where
        S: Stream<Item = LeaseItem> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(BufferState::Lines(VecDeque::new())));
        tokio::spawn(start_worker(state.clone(), stream, scrollback));
        TailStream { state }
    }
}

async fn start_worker<S>(state: Arc<Mutex<BufferState>>, stream: S, scrollback: usize)
where
    S: Stream<Item = LeaseItem> + Send + 'static,
{
    pin!(stream);
    loop {
        let item = stream.next().await;
        let end = item.is_none();
        let signal_rx = {
            let mut lock = state.lock().expect("state");
            match &mut *lock {
                BufferState::Lines(lines) => {
                    if lines.len() == scrollback {
                        lines.drain(..1);
                    }
                    lines.push_back(item);
                    if end {
                        break;
                    }
                    continue;
                }
                BufferState::Pending {
                    future_rx,
                    signal_tx,
                    worker,
                } => {
                    let Some((future_tx, signal_rx)) = worker.take() else {
                        warn! { "The {} is not waiting on the worker", TailStream::type_name() };
                        return;
                    };
                    let Ok(()) = future_tx.send(item) else {
                        warn! { "The {}'s future_rx was dropped", TailStream::type_name() };
                        return;
                    };
                    signal_rx
                }
            }
        };
        // Await after releasing the lock
        let _ = signal_rx.await;
        if end {
            break;
        }
    }
}

impl Stream for TailStream {
    type Item = LeaseItem;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.lock().expect("state");
        loop {
            let state = &mut *state;
            let ProcessResult { new_state, result } = process_state(cx, state);
            if let Some(new_state) = new_state {
                *state = new_state;
            }
            if let Some(result) = result {
                return result;
            }
        }
    }
}

fn process_state(cx: &mut Context<'_>, state: &mut BufferState) -> ProcessResult {
    match state {
        BufferState::Lines(items) => {
            let next = if items.is_empty() {
                None
            } else {
                items.drain(..1).next()
            };
            match next {
                Some(item) => ProcessResult {
                    new_state: None,
                    result: Some(Poll::Ready(item)),
                },
                None => {
                    let (future_tx, future_rx) = oneshot::channel();
                    let (signal_tx, signal_rx) = oneshot::channel();
                    ProcessResult {
                        new_state: Some(BufferState::Pending {
                            future_rx,
                            signal_tx: Some(signal_tx),
                            worker: Some((future_tx, signal_rx)),
                        }),
                        result: None,
                    }
                }
            }
        }
        BufferState::Pending {
            future_rx,
            signal_tx,
            worker,
        } => match future_rx.poll_unpin(cx) {
            Poll::Ready(Ok(data)) => {
                assert!(worker.is_none());
                if let Some(signal_tx) = signal_tx.take() {
                    if signal_tx.send(()).is_err() {
                        warn! { "The worker has stopped without waiting for signal_tx" };
                    }
                } else {
                    warn! { "The worker is not expecting the {} to continue", TailStream::type_name() };
                }
                ProcessResult {
                    new_state: Some(BufferState::Lines(VecDeque::new())),
                    result: Some(Poll::Ready(data)),
                }
            }
            Poll::Ready(Err(oneshot::error::RecvError { .. })) => ProcessResult {
                new_state: None,
                result: Some(Poll::Ready(Some(LeaseItem::Error(
                    ErrorKind::BrokenPipe.into(),
                )))),
            },
            Poll::Pending => ProcessResult {
                new_state: None,
                result: Some(Poll::Pending),
            },
        },
    }
}

struct ProcessResult {
    new_state: Option<BufferState>,
    result: Option<Poll<Option<LeaseItem>>>,
}

enum BufferState {
    /// There are buffered lines.
    Lines(VecDeque<Option<LeaseItem>>),

    /// Waiting for some lines to be read
    Pending {
        future_rx: oneshot::Receiver<Option<LeaseItem>>,
        signal_tx: Option<oneshot::Sender<()>>,
        worker: Option<(oneshot::Sender<Option<LeaseItem>>, oneshot::Receiver<()>)>,
    },
}

#[cfg(test)]
mod tests {
    use std::future::ready;
    use std::time::Duration;

    use futures::StreamExt;
    use terrazzo_pty::lease::LeaseItem;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use crate::processes::tail::TailStream;

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
                ready(end)
            })
            .map(|i| LeaseItem::Data(i.to_string().into_bytes()));
        let mut tail_stream = TailStream::new(stream, 5);
        let _ = end_rx.await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        let data = tail_stream
            .take(10)
            .map(|item| match item {
                LeaseItem::EOS => "EOS".to_string(),
                LeaseItem::Data(data) => String::from_utf8(data).unwrap(),
                LeaseItem::Error(error) => format!("Error: {error}"),
            })
            .collect::<Vec<_>>()
            .await;
        assert_eq!(vec!["996", "997", "998", "999"], data);
    }
}
