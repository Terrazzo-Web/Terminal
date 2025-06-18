use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::prelude::OrElseLog as _;
use terrazzo::prelude::diagnostics;
use web_sys::Response;

use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use self::diagnostics::info_span;
use self::diagnostics::warn;
use super::DISPATCHERS;
use crate::api::TerminalAddress;
use crate::api::client::request::BASE_URL;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::ThenRequest as _;
use crate::api::client::request::send_request;
use crate::api::client::request::set_correlation_id;
use crate::api::client::request::set_headers;
use crate::api::client::request::set_json_body;
use crate::terminal_id::TerminalId;

/// Sends a request to close the process.
#[nameth]
pub async fn close(terminal: &TerminalAddress, correlation_id: Option<String>) {
    let terminal_id = &terminal.id;
    async move {
        let _: Response = send_request(
            Method::POST,
            format!("{BASE_URL}/stream/{CLOSE}"),
            set_headers(set_correlation_id(correlation_id.as_deref()))
                .then(set_json_body(terminal)?),
        )
        .await?;
        debug!("End");
        Ok(())
    }
    .instrument(info_span!("Close", %terminal_id))
    .await
    .unwrap_or_else(|error: CloseError| warn!("Failed to close the terminal: {error}"))
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum CloseError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    JsonSerializationError(#[from] serde_json::Error),
}

pub fn drop_dispatcher(terminal_id: &TerminalId) -> Option<Arc<str>> {
    debug!("Drop dispatcher");
    let mut dispatchers_lock = DISPATCHERS.lock().or_throw("DISPATCHERS");
    let dispatchers = dispatchers_lock.as_mut()?;
    dispatchers.map.remove(terminal_id);

    // The pipe closes when the last terminal closes and StreamDispatchers is dropped.
    if !dispatchers.map.is_empty() {
        return None;
    }

    return dispatchers_lock.take().map(|d| d.correlation_id);
}
