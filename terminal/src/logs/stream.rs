use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use server_fn::codec::StreamingText;
use server_fn::codec::TextStream;
use terrazzo::server;

#[server(protocol = Http<Json, StreamingText>)]
pub async fn stream() -> Result<TextStream<ServerFnError>, ServerFnError> {
    imp::stream_impl().await
}

#[cfg(feature = "server")]
mod imp {
    use futures::StreamExt as _;
    use server_fn::ServerFnError;
    use server_fn::codec::TextStream;

    use crate::logs::event::LogEvent;
    use crate::logs::state::LogState;

    pub(super) async fn stream_impl() -> Result<TextStream<ServerFnError>, ServerFnError> {
        let subscription = LogState::get().subscribe();
        let stream = futures::stream::unfold(subscription, |mut subscription| async move {
            let next = if let Some(event) = subscription.backlog.pop_front() {
                Some(event)
            } else {
                subscription.receiver.recv().await
            }?;
            Some((serialize_log_event(&next), subscription))
        });
        Ok(TextStream::new(stream.map(Ok)))
    }

    fn serialize_log_event(event: &LogEvent) -> String {
        serde_json::to_string(event).unwrap_or_else(|error| {
            serde_json::to_string(&LogEvent {
                id: event.id,
                level: event.level,
                message: format!("Failed to serialize log event: {error}"),
                timestamp_ms: event.timestamp_ms,
            })
            .expect("serialize fallback log event")
        }) + "\n"
    }
}

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use futures::StreamExt as _;
    use tracing::info;
    use tracing::warn;

    use crate::logs::stream::stream;
    use crate::logs::tests::TestGuard;

    #[tokio::test]
    async fn stream_logs_replays_backlog_and_then_live_events() {
        let guard = TestGuard::get();
        guard.with_test_subscriber(|| {
            info!("backlog");
        });

        let mut stream = stream().await.expect("stream").into_inner();
        let backlog = stream.next().await.expect("item").expect("data");
        {
            let expected = r#""backlog","#;
            assert!(
                backlog.contains(expected),
                "Expected {backlog} contains {expected}"
            );
        }

        guard.with_test_subscriber(|| {
            warn!("live");
        });

        let live = stream.next().await.expect("item").expect("data");
        {
            let expected = r#""live","#;
            assert!(
                live.contains(expected),
                "Expected {live} contains {expected}"
            );
        }
    }
}
