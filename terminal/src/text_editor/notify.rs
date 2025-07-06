use std::sync::Arc;

use nameth::nameth;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use server_fn::Websocket;
use server_fn::codec::JsonEncoding;
use terrazzo::server;

use super::file_path::FilePath;
use crate::api::client_address::ClientAddress;

mod event_handler;
pub mod service;
pub mod ui;
mod watcher;

#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
#[nameth]
pub async fn notify(
    request: BoxedStream<NotifyRequest, ServerFnError>,
) -> Result<BoxedStream<NotifyResponse, ServerFnError>, ServerFnError> {
    use crate::backend::client_service::notify::notify_hybrid;
    Ok(notify_hybrid(request.into())?.into())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum NotifyRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "S"))]
    Start {
        #[cfg_attr(not(feature = "diagnostics"), serde(rename = "r"))]
        remote: ClientAddress,
    },
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "W"))]
    Watch {
        #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
        full_path: FilePath<Arc<str>>,
    },
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "U"))]
    UnWatch {
        #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
        full_path: FilePath<Arc<str>>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct NotifyResponse {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
    pub path: String,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "k"))]
    pub kind: EventKind,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub enum EventKind {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "C"))]
    Create,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "M"))]
    Modify,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "D"))]
    Delete,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    Error,
}
