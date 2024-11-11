use named::named;
use named::NamedEnumValues as _;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use tracing::Instrument;
use web_sys::Headers;
use web_sys::Response;

use super::send_request;
use super::Method;
use super::SendRequestError;
use super::BASE_URL;
use super::DISPATCHERS;
use crate::api::client::stream::ShutdownPipe;
use crate::api::client::stream::StreamDispatchers;
use crate::api::CORRELATION_ID;
use crate::terminal_id::TerminalId;

#[named]
pub async fn close(terminal_id: &TerminalId) -> Result<(), CloseError> {
    async {
        let close_pipe = drop_dispatcher(terminal_id);
        let _: Response = send_request(
            Method::POST,
            format!("{BASE_URL}/stream/{CLOSE}/{terminal_id}"),
            move |request| {
                if let Some(correlation_id) = close_pipe {
                    let headers = Headers::new().expect("Headers::new()");
                    headers
                        .set(CORRELATION_ID, &correlation_id)
                        .expect(CORRELATION_ID);
                    request.set_headers(headers.as_ref());
                }
            },
        )
        .await?;
        return Ok(());
    }
    .instrument(info_span!("Close", %terminal_id))
    .await
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum CloseError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}

pub fn drop_dispatcher(terminal_id: &TerminalId) -> Option<String> {
    debug!("Drop dispatcher");
    let mut dispatchers_lock = DISPATCHERS.lock().unwrap();
    let dropped_dispatchers: Option<StreamDispatchers> =
        if let Some(dispatchers) = &mut *dispatchers_lock {
            dispatchers.map.remove(terminal_id);
            if dispatchers.map.is_empty() {
                // The pipe closes when the last terminal closes and StreamDispatchers is dropped.
                dispatchers_lock.take()
            } else {
                None
            }
        } else {
            None
        };

    // Make sure the dispatchers are dropped after the lock is released.
    drop(dispatchers_lock);
    dropped_dispatchers.map(|d| {
        debug!("Send pipe shutdown");
        if let ShutdownPipe::Signal(signal) = d.shutdown_pipe {
            match signal.send(()) {
                Ok(()) => info!("Closed"),
                Err(()) => debug!("Already shutdown"),
            }
        } else {
            warn!("Pipe was still pending")
        }
        d.correlation_id
    })
}
