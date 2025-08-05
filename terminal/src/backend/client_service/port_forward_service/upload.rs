use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::net::TcpStream;
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

/// Upload data from listener
pub async fn upload(
    server: &Arc<Server>,
    download_stream: impl RequestDataStream,
) -> Result<GrpcStream, GrpcStreamError<UploadLocalError>> {
    stream::<GetUploadStream>(server, download_stream)
        .instrument(info_span!("PortForward Upload"))
        .await
}

struct GetUploadStream;

impl GetLocalStream for GetUploadStream {
    type Error = UploadLocalError;

    async fn get_tcp_stream(endpoint_id: EndpointId) -> Result<TcpStream, Self::Error> {
        let EndpointId { host, port } = endpoint_id;
        let tcp_stream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(UploadLocalError::Connect)?;
        let () = tcp_stream
            .set_nodelay(true)
            .map_err(UploadLocalError::SetNodelay)?;
        Ok(tcp_stream)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum UploadLocalError {
    #[error("[{n}] Failed to connect: {0}", n = self.name())]
    Connect(std::io::Error),

    #[error("[{n}] Failed to set TCP_NODELAY option to true: {0}", n = self.name())]
    SetNodelay(std::io::Error),
}

impl From<UploadLocalError> for Status {
    fn from(error: UploadLocalError) -> Self {
        let code = match error {
            UploadLocalError::Connect { .. } => tonic::Code::Aborted,
            UploadLocalError::SetNodelay { .. } => tonic::Code::Internal,
        };
        Self::new(code, error.to_string())
    }
}
