use std::future::ready;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;

use axum::body::Body;
use axum::response::IntoResponse;
use futures::Stream;
use futures::StreamExt as _;
use futures::stream::once;
use pin_project::pin_project;
use scopeguard::defer;
use scopeguard::guard;
use static_assertions::const_assert;
use terrazzo::autoclone;
use terrazzo::axum;
use terrazzo_pty::lease::LeaseItem;
use tracing::Span;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::trace;
use tracing_futures::Instrument as _;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use super::registration::PingTimeoutError;
use crate::api::KEEPALIVE_TTL_HEADER;
use crate::api::NEWLINE;
use crate::api::server::correlation_id::CorrelationId;
use crate::api::server::stream::registration::Registration;
use crate::api::shared::terminal_schema::Chunk;
use crate::backend::client_service::terminal_service;

pub const PIPE_TTL: Duration = if cfg!(feature = "concise-traces") {
    Duration::from_secs(3600)
} else {
    Duration::from_secs(5)
};

pub const KEEPALIVE_TTL: Duration = if cfg!(feature = "concise-traces") {
    Duration::from_secs(20)
} else {
    Duration::from_secs(3)
};

#[autoclone]
pub fn pipe(server: Arc<Server>, correlation_id: CorrelationId) -> impl IntoResponse {
    let span = info_span!("Pipe", %correlation_id);
    let _span = span.clone().entered();
    info!("Start");
    let (rx, keepalive) = Registration::set(correlation_id.clone());
    struct RxDropped;
    let rx_dropped = Arc::new(guard(RxDropped, move |_| {
        let _span = span.entered();
        drop(Registration::take_if(&correlation_id));
        info!("End");
    }));
    let rx = rx.flat_map_unordered(None, move |(terminal_address, lease)| {
        let _rx_dropped = rx_dropped.clone();
        let terminal_id = terminal_address.id;
        let client_address = terminal_address.via;
        let span = tracing::info_span!("Lease", %terminal_id);

        // Debug logs
        #[cfg(debug_assertions)]
        let lease = lease.inspect(|chunk| {
            use nameth::NamedEnumValues as _;
            match chunk {
                LeaseItem::EOS => tracing::debug!("{}", chunk.name()),
                LeaseItem::Data(data) => assert!(!data.is_empty(), "Unexpected empty chunk"),
                LeaseItem::Error(error) => tracing::warn!("Stream failed with: {error}"),
            }
        });

        // Ignore error (logged above) and close stream on failure

        let lease = lease
            // Remove processes when EOS or failure
            .inspect(move |chunk| {
                autoclone!(server, terminal_id);
                match chunk {
                    LeaseItem::EOS | LeaseItem::Error { .. } => {}
                    LeaseItem::Data { .. } => return,
                };
                let task = async move {
                    autoclone!(server, terminal_id, client_address);
                    match self::terminal_service::close::close(
                        &server,
                        &client_address,
                        terminal_id.clone(),
                    )
                    .await
                    {
                        Ok(()) => debug!("Closed {terminal_id}"),
                        Err(error) => debug!("Closing {terminal_id} returned {error}"),
                    }
                };
                tokio::spawn(task.in_current_span());
            });

        // Concat chunks
        let lease = lease
            .ready_chunks(10)
            .flat_map(move |chunks| {
                debug_assert!(!chunks.is_empty(), "Unexpected empty chunks");
                let mut data = vec![];
                for chunk in chunks {
                    if let LeaseItem::Data(chunk) = chunk {
                        data.extend(chunk)
                    } else {
                        return futures::stream::iter([Some(data), None]);
                    }
                }
                trace! { "Streaming {}", String::from_utf8_lossy(&data).escape_default() };
                return futures::stream::iter([Some(data), Some(vec![])]);
            })
            .filter(|chunk| ready(*chunk != Some(vec![])));

        // Serialize as JSON separated by newlines.
        let lease = lease.map(move |data| {
            let terminal_id = terminal_id.clone();
            let mut json = vec![];
            serde_json::to_writer(&mut json, &Chunk { terminal_id, data })?;
            json.push(NEWLINE);
            return Ok::<Vec<u8>, std::io::Error>(json);
        });
        return LeaseClientStream {
            on_drop: LeaseClientStreamDrop { span: span.clone() },
            stream: lease,
        }
        .instrument(span);
    });
    let stream = once(ready(Ok(vec![NEWLINE]))).chain(rx);
    let stream = timeout_stream(stream).take_until(keepalive);
    let stream = stream.in_current_span();
    let body = if tracing::enabled!(tracing::Level::DEBUG) {
        Body::from_stream(stream.inspect(|buffer| {
            if let Ok(buffer) = buffer {
                let str = String::from_utf8_lossy(buffer);
                debug!("Buffer = '{str}'");
                return;
            }
            debug!("Buffer = {buffer:?}")
        }))
    } else {
        Body::from_stream(stream)
    };
    const_assert!(KEEPALIVE_TTL.as_millis() < PIPE_TTL.as_millis());
    return ([(KEEPALIVE_TTL_HEADER, KEEPALIVE_TTL.as_secs())], body);
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
    let _span = info_span!("ClosePipe", %correlation_id).entered();
    info!("Start");
    defer!(info!("End"));
    debug!("Drop the registration");
    drop(Registration::take_if(&correlation_id));
}

pub fn keepalive(
    correlation_id: CorrelationId,
) -> impl Future<Output = Result<(), HttpError<PingTimeoutError>>> {
    let span = info_span!("Keepalive", %correlation_id);
    async move {
        debug!("Start");
        defer!(debug!("Done"));
        let () = Registration::ping_timeout(&correlation_id)?;
        Ok(())
    }
    .instrument(span)
}
