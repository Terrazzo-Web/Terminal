use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::sync::oneshot;
use tonic::Status;
use tracing::Instrument as _;
use tracing::info_span;
use trz_gateway_server::server::Server;

use super::RequestDataStream;
use super::listeners::EndpointId;
use super::stream::GetLocalStream;
use super::stream::GrpcStream;
use super::stream::GrpcStreamError;
use super::stream::stream;

/// Download data from listener
pub async fn download(
    server: &Arc<Server>,
    upload_stream: impl RequestDataStream,
) -> Result<GrpcStream, GrpcStreamError<DownloadLocalError>> {
    stream::<GetDownloadStream>(server, upload_stream)
        .instrument(info_span!("PortForward Download"))
        .await
}

struct GetDownloadStream;

impl GetLocalStream for GetDownloadStream {
    type Error = DownloadLocalError;

    async fn get_tcp_stream(endpoint_id: EndpointId) -> Result<tokio::net::TcpStream, Self::Error> {
        let (future_streams, tx) = {
            let mut listeners = super::listeners::listeners();
            let Some(future_streams) = listeners.get_mut(&endpoint_id) else {
                return Err(DownloadLocalError::StreamsNotRegistered(endpoint_id));
            };
            let (tx, rx) = oneshot::channel();
            (std::mem::replace(future_streams, rx), tx)
        };

        let streams = future_streams
            .await
            .map_err(DownloadLocalError::StreamsNotAvailable)?;
        let mut streams = scopeguard::guard(streams, |streams| {
            let _ = tx.send(streams);
        });
        Ok(streams
            .recv()
            .await
            .ok_or(DownloadLocalError::NoMoreStreams)?)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadLocalError {
    #[error("[{n}] No streams registered under {0:?}", n = self.name())]
    StreamsNotRegistered(EndpointId),

    #[error("[{n}] Failed to get streams: {0}", n = self.name())]
    StreamsNotAvailable(oneshot::error::RecvError),

    #[error("[{n}] No more streams", n = self.name())]
    NoMoreStreams,
}

impl From<DownloadLocalError> for Status {
    fn from(error: DownloadLocalError) -> Self {
        let code = match error {
            DownloadLocalError::StreamsNotRegistered { .. } => tonic::Code::InvalidArgument,
            DownloadLocalError::StreamsNotAvailable { .. } => tonic::Code::FailedPrecondition,
            DownloadLocalError::NoMoreStreams { .. } => tonic::Code::FailedPrecondition,
        };
        Self::new(code, error.to_string())
    }
}
