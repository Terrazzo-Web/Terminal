use std::future::ready;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::Duration;

use autoclone_macro::autoclone;
use axum::body::Body;
use futures::channel::mpsc;
use futures::stream::once;
use futures::Stream;
use futures::StreamExt as _;
use pin_project::pin_project;
use scopeguard::defer;
use scopeguard::guard;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::trace;
use tracing::Span;

use crate::api::server::correlation_id::CorrelationId;
use crate::api::server::stream::registration::Registration;
use crate::api::Chunk;
use crate::api::NEWLINE;
use crate::processes;

const PIPE_TTL: Duration = if cfg!(feature = "concise_traces") {
    Duration::from_secs(3600)
} else {
    Duration::from_secs(5)
};

#[autoclone]
pub fn pipe(correlation_id: CorrelationId) -> Body {
    let span = info_span!("Pipe", %correlation_id);
    let _span = span.clone().entered();
    info!("Start");
    let (tx, rx) = mpsc::channel(10);
    Registration::set(correlation_id.clone(), tx);
    struct RxDropped;
    let rx_dropped = Arc::new(guard(RxDropped, move |_| {
        let _span = span.entered();
        drop(Registration::get_if(&correlation_id));
        info!("End");
    }));
    let rx = rx.flat_map_unordered(None, move |(terminal_id, lease)| {
        let _rx_dropped = rx_dropped.clone();

        #[cfg(debug_assertions)]
        let span = lease.span().clone();
        #[cfg(not(debug_assertions))]
        let span = tracing::info_span!("Lease", %terminal_id);

        // Debug logs
        #[cfg(debug_assertions)]
        let lease = lease.inspect(|chunk| match chunk {
            Ok(chunk) => assert!(!chunk.is_empty(), "Unexpected empty chunk"),
            Err(error) => tracing::warn!("Stream failed with {error}"),
        });

        // Ignore error (logged above) and close stream on failure
        let lease = lease
            .take_while(|chunk| ready(chunk.is_ok()))
            .filter_map(|chunk| ready(chunk.ok()))
            .ready_chunks(10);

        // Concat chunks
        let lease = lease.map(move |chunks| {
            debug_assert!(!chunks.is_empty(), "Unexpected empty chunks");
            let data = chunks.concat();
            trace! { "Streaming {}", String::from_utf8_lossy(&data).escape_default() };
            Some(data)
        });

        // Add tombstone None for EOS
        let lease = lease.chain(once(Box::pin(async move {
            autoclone!(terminal_id);
            match processes::close::close(&terminal_id) {
                Ok(()) => debug!("Closed {terminal_id}"),
                Err(error) => debug!("Closing {terminal_id} returned {error}"),
            };
            return None;
        })));

        // Serialize as JSON separated by newlines.
        let lease = lease.map(move |data| {
            let terminal_id = terminal_id.clone();
            let mut json = vec![];
            serde_json::to_writer(&mut json, &Chunk { terminal_id, data })?;
            json.push(NEWLINE);
            return Ok::<Vec<u8>, std::io::Error>(json);
        });
        return LeaseClientStream {
            on_drop: LeaseClientStreamDrop { span },
            stream: lease,
        };
    });
    let stream = once(ready(Ok(vec![NEWLINE]))).chain(rx);
    let stream = timeout_stream(stream);
    return Body::from_stream(stream);
}

#[autoclone]
fn timeout_stream<I>(stream: impl Stream<Item = I>) -> impl Stream<Item = I> {
    let tick = Arc::new(AtomicBool::new(false));
    let stream = stream.inspect(move |_| {
        autoclone!(tick);
        tick.store(true, SeqCst)
    });
    let (stream, handle) = futures::stream::abortable(stream);
    tokio::spawn(async move {
        loop {
            let () = tokio::time::sleep(PIPE_TTL).await;
            if tick.swap(false, SeqCst) {
                continue;
            }
            handle.abort();
        }
    });
    stream
}

#[pin_project]
struct LeaseClientStream<S> {
    on_drop: LeaseClientStreamDrop,
    #[pin]
    stream: S,
}

impl<S: futures::Stream> futures::Stream for LeaseClientStream<S> {
    type Item = S::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

struct LeaseClientStreamDrop {
    span: Span,
}

impl Drop for LeaseClientStreamDrop {
    fn drop(&mut self) {
        self.span.in_scope(|| info!("End of stream lease"));
    }
}

pub async fn close_pipe(correlation_id: CorrelationId) {
    let _span = info_span!("ClosePipe").entered();
    info!("Start");
    defer!(info!("End"));
    debug!("Drop the registration {correlation_id}");
    drop(Registration::get_if(&correlation_id));
}
