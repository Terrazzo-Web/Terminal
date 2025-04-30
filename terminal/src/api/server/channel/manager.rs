use std::collections::HashMap;
use std::sync::Mutex;

use terrazzo::axum::body::BodyDataStream;
use tokio::sync::oneshot;

use crate::api::server::correlation_id::CorrelationId;

static CHANNELS: Mutex<Option<HashMap<CorrelationId, PendingUploadStream>>> = Mutex::new(None);

pub struct PendingUploadStream {
    pub upload_stream: BodyDataStream,
    pub signal: oneshot::Sender<()>,
}

pub async fn add_upload_stream(correlation_id: CorrelationId, upload_stream: BodyDataStream) {
    let (tx, rx) = oneshot::channel();
    {
        let mut channels = CHANNELS.lock().unwrap();
        let channels = channels.get_or_insert_default();
        channels.insert(
            correlation_id,
            PendingUploadStream {
                upload_stream,
                signal: tx,
            },
        );
    }
    let _end = rx.await;
}

pub fn use_upload_stream(correlation_id: &CorrelationId) -> Option<PendingUploadStream> {
    let mut channels = CHANNELS.lock().unwrap();
    let channels = channels.as_mut()?;
    channels.remove(correlation_id)
}
