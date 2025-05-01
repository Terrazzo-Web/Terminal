use std::sync::Arc;

use futures::TryStream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use prost::bytes::Bytes;
use scopeguard::defer;
use terrazzo::axum;
use terrazzo::axum::body::Body;
use terrazzo::axum::response::Response;
use terrazzo::http::StatusCode;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use super::manager::PendingUploadStream;
use super::manager::use_upload_stream;
use crate::api::server::correlation_id::CorrelationId;

pub async fn download(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    correlation_id: CorrelationId,
) -> Result<Response<Body>, HttpError<DownloadError>> {
    Ok(download_impl(my_client_name, server, correlation_id).await?)
}

async fn download_impl(
    _my_client_name: Option<ClientName>,
    _server: Arc<Server>,
    correlation_id: CorrelationId,
) -> Result<Response<Body>, DownloadError> {
    let PendingUploadStream {
        upload_stream,
        signal,
    } = use_upload_stream(&correlation_id)
        .ok_or_else(move || DownloadError::UploadNotFound { correlation_id })?;
    defer! {
        let _ = signal.send(());
    }
    Ok(Response::new(Body::from_stream(run_terminal_server(
        upload_stream,
    ))))
}

fn run_terminal_server(
    _upload_stream: impl TryStream<Ok = Bytes, Error = axum::Error>,
) -> impl TryStream<Ok = Bytes, Error = DownloadError> + Send {
    futures::stream::empty()
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("[{n}] The upload stream {correlation_id} was not found", n = self.name())]
    UploadNotFound { correlation_id: CorrelationId },
}

impl IsHttpError for DownloadError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UploadNotFound { .. } => StatusCode::NOT_FOUND,
        }
    }
}
