use futures::TryStreamExt as _;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::axum::body::Body;
use terrazzo::axum::extract::Path;
use terrazzo::http::StatusCode;
use tracing::Instrument as _;
use tracing::debug_span;
use tracing::trace;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;

use crate::processes;
use crate::processes::write::WriteChunkError;
use crate::terminal_id::TerminalId;

pub async fn write(
    Path(terminal_id): Path<TerminalId>,
    data: Body,
) -> Result<(), HttpError<WriteError>> {
    let span = debug_span!("Write", %terminal_id);
    span.in_scope(|| trace!("Start"));
    defer!(span.in_scope(|| trace!("End")));
    let response = data
        .into_data_stream()
        .map_err(WriteError::RequestBody)
        .try_for_each(move |data| {
            let terminal_id = terminal_id.clone();
            async move { Ok(processes::write::write_chunk(&terminal_id, &data).await?) }
        })
        .instrument(span.clone())
        .await;
    return Ok(response?);
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    #[error("[{n}] Failed get request body: {0}", n = self.name())]
    RequestBody(terrazzo::axum::Error),

    #[error("[{n}] Failed write chunk: {0}", n = self.name())]
    Chunk(#[from] WriteChunkError),
}

impl IsHttpError for WriteError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::RequestBody { .. } => StatusCode::BAD_REQUEST,
            Self::Chunk(error) => error.status_code(),
        }
    }
}
