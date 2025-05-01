use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::axum::body::BodyDataStream;
use terrazzo::http::StatusCode;
use tokio::sync::oneshot;
use tracing::Instrument;
use tracing::debug;
use tracing::warn;
use trz_gateway_common::http_error::IsHttpError;

use crate::api::server::correlation_id::CorrelationId;

static CHANNELS: Mutex<Option<HashMap<CorrelationId, PendingUploadStream>>> = Mutex::new(None);
const MAX_PENDING_CHANNELS: usize = if cfg!(debug_assertions) { 3 } else { 10 };
const PENDING_CHANNEL_TIMEOUT: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(2)
} else {
    Duration::from_secs(15)
};

pub struct PendingUploadStream {
    pub upload_stream: BodyDataStream,
    pub signal: oneshot::Sender<()>,
}

pub async fn add_upload_stream(
    correlation_id: CorrelationId,
    upload_stream: BodyDataStream,
) -> Result<(), PendingChannelError> {
    // Add a delay to simulate upload stream correlation ID not found transient error.
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let (tx, rx) = oneshot::channel();
    {
        let mut channels = CHANNELS.lock().unwrap();
        let channels = channels.get_or_insert_default();
        if channels.len() >= MAX_PENDING_CHANNELS {
            return Err(PendingChannelError::MaxPendingChannelsExceeded);
        }
        channels.insert(
            correlation_id.clone(),
            PendingUploadStream {
                upload_stream,
                signal: tx,
            },
        );
    }
    let timeout = tokio::spawn(
        async move {
            tokio::time::sleep(PENDING_CHANNEL_TIMEOUT).await;
            let mut channels = CHANNELS.lock().unwrap();
            let Some(channels) = &mut *channels else {
                return;
            };
            if let Some(_) = channels.remove(&correlation_id) {
                warn!("A pending channel timed out");
            } else {
                // This should not show since we cancel the timeout task on success
                debug!("A pending channel did not time out");
            }
        }
        .in_current_span(),
    );
    let rx = rx.await;
    timeout.abort();
    Ok(rx.map_err(|_: oneshot::error::RecvError| PendingChannelError::PendingChannelTimeout)?)
}

pub fn use_upload_stream(correlation_id: &CorrelationId) -> Option<PendingUploadStream> {
    let mut channels = CHANNELS.lock().unwrap();
    let channels = channels.as_mut()?;
    channels.remove(correlation_id)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PendingChannelError {
    #[error("[{n}] Exceeded the maximum number of pending channnels = {MAX_PENDING_CHANNELS}", n = self.name())]
    MaxPendingChannelsExceeded,

    #[error("[{n}] The pending channel was never consumed", n = self.name())]
    PendingChannelTimeout,
}

impl IsHttpError for PendingChannelError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::MaxPendingChannelsExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::PendingChannelTimeout => StatusCode::REQUEST_TIMEOUT,
        }
    }
}
