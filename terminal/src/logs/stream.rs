use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use server_fn::codec::StreamingText;
use server_fn::codec::TextStream;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

#[server(protocol = Http<Json, StreamingText>)]
pub async fn stream(
    remote: Option<ClientAddress>,
) -> Result<TextStream<ServerFnError>, ServerFnError> {
    imp::stream_impl(remote).await
}

#[cfg(feature = "server")]
mod imp {
    use std::pin::Pin;

    use futures::Stream;
    use futures::TryStreamExt as _;
    use server_fn::BoxedStream;
    use server_fn::ServerFnError;
    use server_fn::codec::TextStream;

    use crate::api::client_address::ClientAddress;
    use crate::backend::client_service::logs_service::dispatch::logs_dispatch;
    use crate::backend::protos::terrazzo::logs::LogsRequest;
    use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;
    use crate::logs::event::LogEvent;

    pub(super) async fn stream_impl(
        remote: Option<ClientAddress>,
    ) -> Result<TextStream<ServerFnError>, ServerFnError> {
        let request = LogsRequest {
            address: remote.map(|remote| ClientAddressProto::of(&remote)),
        };
        let stream = logs_dispatch(request)
            .await
            .map(BoxedStream::from)
            .map_err(ServerFnError::new)?;
        let stream: Pin<Box<dyn Stream<Item = _> + Send>> = stream.into();
        let stream = stream.map_ok(|event| serialize_log_event(&event));
        Ok(TextStream::new(stream))
    }

    fn serialize_log_event(event: &LogEvent) -> String {
        serde_json::to_string(event).unwrap_or_else(|error| {
            serde_json::to_string(&LogEvent {
                id: event.id,
                level: event.level,
                message: format!("Failed to serialize log event: {error}"),
                timestamp_ms: event.timestamp_ms,
                file: None,
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

        let mut stream = stream(None).await.expect("stream").into_inner();
        let backlog = stream.next().await.expect("item").expect("data");
        assert!(
            backlog.contains("backlog"),
            "Expected {backlog} contains backlog"
        );

        guard.with_test_subscriber(|| {
            warn!("live");
        });

        let live = stream.next().await.expect("item").expect("data");
        assert!(live.contains("live"), "Expected {live} contains live");
    }
}
