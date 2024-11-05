use axum::body::Body;
use axum::extract::Path;
use axum::response::Response;
use futures::TryStreamExt as _;
use http::StatusCode;
use scopeguard::defer;
use tracing::debug_span;
use tracing::trace;
use tracing::Instrument;

use super::into_error;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn write(Path(terminal_id): Path<TerminalId>, data: Body) -> Result<(), Response> {
    let span = debug_span!("Write", %terminal_id);
    span.in_scope(|| trace!("Start"));
    defer!(span.in_scope(|| trace!("End")));
    data.into_data_stream()
        .map_err(into_error(StatusCode::BAD_REQUEST))
        .try_for_each(move |data| {
            let terminal_id = terminal_id.clone();
            async move {
                processes::write::write(&terminal_id, &data)
                    .await
                    .map_err(into_error(StatusCode::INTERNAL_SERVER_ERROR))
            }
        })
        .instrument(span.clone())
        .await
}
