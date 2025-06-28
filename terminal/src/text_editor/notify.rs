use std::sync::Arc;

use nameth::nameth;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use server_fn::Websocket;
use server_fn::codec::JsonEncoding;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

mod event_handler;
mod service;

#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
#[nameth]
async fn notify(
    request: BoxedStream<NotifyRequest, ServerFnError>,
) -> Result<BoxedStream<NotifyResponse, ServerFnError>, ServerFnError> {
    service::notify(request).await
}

#[derive(serde::Serialize, serde::Deserialize)]
enum NotifyRequest {
    Start { remote: ClientAddress },
    Watch { path: Arc<str> },
    UnWatch { path: Arc<str> },
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct NotifyResponse {
    path: String,
    kind: EventKind,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
enum EventKind {
    Create,
    Modify,
    Delete,
    Error,
}
