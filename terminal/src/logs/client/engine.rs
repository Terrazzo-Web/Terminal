use std::collections::VecDeque;
use std::sync::Arc;

use futures::StreamExt as _;
use futures::future::AbortHandle;
use futures::future::Abortable;
use scopeguard::defer;
use server_fn::ServerFnError;
use server_fn::codec::TextStream;
use terrazzo::prelude::XSignal;
use terrazzo::prelude::diagnostics;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::debug;
use self::diagnostics::warn;
use super::ndjson::NdjsonBuffer;
use crate::logs::event::LogEvent;

const MAX_LOGS: usize = if cfg!(debug_assertions) { 1000 } else { 25 };

pub struct LogsEngine {
    logs: XSignal<Arc<VecDeque<ClientLogEvent>>>,
    abort_handle: AbortHandle,
}

impl LogsEngine {
    pub fn new() -> Self {
        let logs = XSignal::new("log-events", Arc::default());
        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        let consume_stream = {
            let logs = logs.clone();
            async move {
                let Ok(stream) = crate::logs::stream::stream()
                    .await
                    .inspect_err(|error| warn!("Failed to start log stream: {error}"))
                else {
                    return;
                };
                consume_stream(logs, stream).await;
            }
        };
        spawn_local(async move {
            match Abortable::new(consume_stream, abort_registration).await {
                Ok(()) => debug!("Logs stream finished"),
                Err(_) => debug!("Logs stream aborted"),
            }
        });

        Self { logs, abort_handle }
    }

    pub fn logs(&self) -> XSignal<Arc<VecDeque<ClientLogEvent>>> {
        self.logs.clone()
    }
}

impl Drop for LogsEngine {
    fn drop(&mut self) {
        debug!("Dropping LogsEngine, aborting log stream");
        self.abort_handle.abort();
    }
}

async fn consume_stream(
    logs: XSignal<Arc<VecDeque<ClientLogEvent>>>,
    stream: TextStream<ServerFnError>,
) {
    debug!("Start");
    defer!(debug!("End"));
    let mut parser = NdjsonBuffer::default();
    let mut stream = stream.into_inner();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                let mut new_logs = vec![];
                for event in parser.push_chunk(&chunk) {
                    match event {
                        Ok(event) => new_logs.push(ClientLogEvent::new(event)),
                        Err(error) => warn!("Failed to parse log stream line: {error}"),
                    }
                }
                if new_logs.is_empty() {
                    continue;
                }
                logs.update(|current| {
                    let mut current = current.as_ref().clone();
                    current.extend(new_logs);
                    if current.len() > MAX_LOGS {
                        let start = current.len().saturating_sub(20);
                        current.drain(..start);
                    }
                    Some(Arc::new(current))
                });
            }
            Err(error) => {
                warn!("Log stream failed: {error}");
                return;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientLogEvent {
    event: LogEvent,
    pub received_at_ms: u64,
}

impl ClientLogEvent {
    pub(super) fn new(event: LogEvent) -> Self {
        Self {
            event,
            received_at_ms: web_sys::js_sys::Date::now() as u64,
        }
    }
}

impl std::ops::Deref for ClientLogEvent {
    type Target = LogEvent;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
