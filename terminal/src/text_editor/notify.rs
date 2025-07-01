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
pub mod ui;

#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
#[nameth]
pub async fn notify(
    request: BoxedStream<NotifyRequest, ServerFnError>,
) -> Result<BoxedStream<NotifyResponse, ServerFnError>, ServerFnError> {
    service::notify(request).await
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum NotifyRequest {
    Start { remote: ClientAddress },
    Watch { full_path: Arc<str> },
    UnWatch { full_path: Arc<str> },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct NotifyResponse {
    pub path: String,
    pub kind: EventKind,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub enum EventKind {
    Create,
    Modify,
    Delete,
    Error,
}
