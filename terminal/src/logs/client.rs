#![cfg(feature = "client")]

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use futures::StreamExt as _;
use server_fn::ServerFnError;
use server_fn::codec::TextStream;
use terrazzo::prelude::XSignal;
use terrazzo::prelude::diagnostics;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::warn;
use super::event::LogEvent;

pub fn logs() -> XSignal<Arc<Vec<ClientLogEvent>>> {
    static LOGS: OnceLock<XSignal<Arc<Vec<ClientLogEvent>>>> = OnceLock::new();
    LOGS.get_or_init(|| XSignal::new("log-events", Arc::default()))
        .clone()
}

pub fn ensure_started() {
    static STARTED: OnceLock<Mutex<bool>> = OnceLock::new();
    let started = STARTED.get_or_init(Mutex::default);
    let mut started = started.lock().expect("log stream started");
    if *started {
        return;
    }
    *started = true;
    drop(started);

    let logs = logs();
    spawn_local(async move {
        let Ok(stream) = super::stream::stream()
            .await
            .inspect_err(|error| warn!("Failed to start log stream: {error}"))
        else {
            return;
        };
        consume_stream(logs, stream).await;
    });
}

async fn consume_stream(
    logs: XSignal<Arc<Vec<ClientLogEvent>>>,
    stream: TextStream<ServerFnError>,
) {
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
    pub id: u64,
    pub level: super::event::LogLevel,
    pub message: String,
    pub timestamp_ms: u64,
    pub received_at_ms: u64,
}

impl ClientLogEvent {
    fn new(event: LogEvent) -> Self {
        Self {
            id: event.id,
            level: event.level,
            message: event.message,
            timestamp_ms: event.timestamp_ms,
            received_at_ms: web_sys::js_sys::Date::now() as u64,
        }
    }
}

#[derive(Default)]
struct NdjsonBuffer {
    pending: String,
}

impl NdjsonBuffer {
    fn push_chunk(&mut self, chunk: &str) -> Vec<Result<LogEvent, serde_json::Error>> {
        self.pending.push_str(chunk);

        let mut lines = vec![];
        while let Some(newline) = self.pending.find('\n') {
            let line = self.pending[..newline].to_owned();
            self.pending.drain(..=newline);
            if line.is_empty() {
                continue;
            }
            lines.push(serde_json::from_str(&line));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::NdjsonBuffer;
    use crate::logs::event::LogEvent;
    use crate::logs::event::LogLevel;

    #[test]
    fn splits_lines_and_parses_ndjson_chunks() {
        let event1 = serde_json::to_string(&LogEvent {
            id: 1,
            level: LogLevel::Info,
            message: "first".to_owned(),
            timestamp_ms: 11,
        })
        .expect("event1");
        let event2 = serde_json::to_string(&LogEvent {
            id: 2,
            level: LogLevel::Warn,
            message: "second".to_owned(),
            timestamp_ms: 22,
        })
        .expect("event2");

        let mut parser = NdjsonBuffer::default();

        let first = parser.push_chunk(&(event1.clone() + "\n" + &event2[..8]));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].as_ref().expect("parsed").message, "first");

        let second = parser.push_chunk(&format!("{}\n", &event2[8..]));
        assert_eq!(second.len(), 1);
        let second = second
            .into_iter()
            .next()
            .expect("second line")
            .expect("parsed");
        assert_eq!(second.id, 2);
        assert_eq!(second.level, LogLevel::Warn);
        assert_eq!(second.message, "second");
        assert_eq!(second.timestamp_ms, 22);
    }
}
