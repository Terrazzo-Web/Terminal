#![cfg(feature = "terminal")]

use std::collections::HashMap;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::task::Poll;
use std::task::ready;

use futures::FutureExt as _;
use futures::Stream;
use futures::channel::oneshot;
use pin_project::pin_project;
use pin_project::pinned_drop;
use terrazzo_pty::lease::LeaseItem;
use terrazzo_pty::lease::ProcessOutputLease;
use tracing::debug;

use crate::api::shared::terminal_schema::STREAMING_WINDOW_SIZE;
use crate::terminal_id::TerminalId;

fn streams() -> MutexGuard<'static, HashMap<TerminalId, Arc<Mutex<ThrottlingState>>>> {
    static STREAMS: OnceLock<Mutex<HashMap<TerminalId, Arc<Mutex<ThrottlingState>>>>> =
        OnceLock::new();
    STREAMS.get_or_init(Mutex::default).lock().expect("streams")
}

pub fn ack(terminal_id: &TerminalId, ack: usize) {
    let Some(throttling_state) = streams().get(terminal_id).cloned() else {
        return;
    };
    let mut throttling_state = throttling_state.lock().expect("throttling_state");
    throttling_state.ack -= ack;
    let Some(signal) = throttling_state.signal.take() else {
        // Acks are sent at 1/2 window size, so it's possible the acks are sent before the backend is throttled.
        debug!("Missing signal to ack");
        return;
    };
    debug!("Found signal to ack");
    let _ = signal.send(());
}

#[pin_project(PinnedDrop)]
pub struct ThrottleProcessOutput {
    terminal_id: TerminalId,
    state: Arc<Mutex<ThrottlingState>>,
    #[pin]
    stream: ProcessOutputLease,
}

#[derive(Default)]
struct ThrottlingState {
    ack: usize,
    signal: Option<oneshot::Sender<()>>,
    throttled: Option<oneshot::Receiver<()>>,
}

#[pinned_drop]
impl PinnedDrop for ThrottleProcessOutput {
    fn drop(self: Pin<&mut Self>) {
        streams().remove(&self.terminal_id);
    }
}

impl ThrottleProcessOutput {
    pub fn new(terminal_id: TerminalId, stream: ProcessOutputLease) -> Self {
        let state: Arc<Mutex<ThrottlingState>> = Default::default();
        streams().insert(terminal_id.clone(), state.clone());
        Self {
            terminal_id,
            state,
            stream,
        }
    }
}

impl Stream for ThrottleProcessOutput {
    type Item = <ProcessOutputLease as Stream>::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut state = this.state.lock().expect("throttling state");

        // Check if the stream is in throttled state.
        if let Some(mut throttled) = state.throttled.take() {
            match throttled.poll_unpin(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(oneshot::Canceled)) => {
                    return Poll::Ready(Some(LeaseItem::Error(std::io::Error::new(
                        ErrorKind::BrokenPipe,
                        "Process output throtting signal got canceled",
                    ))));
                }
                Poll::Pending => {
                    // Continue waiting on the throttle state.
                    state.throttled = Some(throttled);
                    return Poll::Pending;
                }
            }
        }

        let result = ready!(this.stream.poll_next(cx));
        if let Some(LeaseItem::Data(bytes)) = &result {
            state.ack += bytes.len();

            // Enter throttled state if too many bytes were sent since last ack.
            if state.ack >= STREAMING_WINDOW_SIZE {
                let (signal, throttled) = oneshot::channel();
                state.signal = Some(signal);
                state.throttled = Some(throttled);
            }
        }
        return Poll::Ready(result);
    }
}
