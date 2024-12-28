use futures::FutureExt as _;
use futures::TryFutureExt as _;
use named::named;
use named::NamedEnumValues as _;
use terrazzo::prelude::OrElseLog as _;
use tracing::debug;
use tracing::info_span;
use tracing::Instrument as _;
use web_sys::Headers;
use web_sys::Response;

use super::send_request;
use super::warn;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use super::DISPATCHERS;
use crate::api::CORRELATION_ID;
use crate::terminal_id::TerminalId;

/// Sends a request to close the process.
#[named]
pub fn close(
    terminal_id: TerminalId,
    correlation_id: Option<String>,
) -> impl std::future::Future<Output = ()> {
    send_request(
        Method::POST,
        format!("{BASE_URL}/stream/{CLOSE}/{terminal_id}"),
        move |request| {
            debug!("Start");
            if let Some(correlation_id) = correlation_id {
                let headers = Headers::new().or_throw("Headers::new()");
                headers
                    .set(CORRELATION_ID, &correlation_id)
                    .or_throw(CORRELATION_ID);
                request.set_headers(headers.as_ref());
            }
        },
    )
    .map(|response| {
        debug!("End");
        let _: Response = response?;
        Ok(())
    })
    .unwrap_or_else(|error: CloseError| warn!("Failed to close the terminal: {error}"))
    .instrument(info_span!("Close", %terminal_id))
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum CloseError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}

pub fn drop_dispatcher(terminal_id: &TerminalId) -> Option<String> {
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
