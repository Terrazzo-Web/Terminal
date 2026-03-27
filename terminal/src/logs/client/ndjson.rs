use crate::logs::event::LogEvent;

#[derive(Default)]
pub(super) struct NdjsonBuffer {
    pending: String,
}

impl NdjsonBuffer {
    pub(super) fn push_chunk(&mut self, chunk: &str) -> Vec<Result<LogEvent, serde_json::Error>> {
        self.pending.push_str(chunk);

        let mut lines = vec![];
        while let Some(newline) = self.pending.find('\n') {
            let line = self.pending[..newline].to_owned();
            self.pending.drain(..=newline);
            if line.is_empty() {
                continue;
            }
            lines.push(serde_json::from_str::<LogEvent>(&line));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use crate::logs::event::LogEvent;
    use crate::logs::event::LogLevel;

    use super::NdjsonBuffer;

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
