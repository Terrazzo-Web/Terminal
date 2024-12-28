use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use http::StatusCode;
use tracing::info_span;
use tracing::Instrument;

use super::into_error;
use crate::api::Size;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn resize(
    Path(terminal_id): Path<TerminalId>,
    Json(Size { rows, cols }): Json<Size>,
) -> Result<(), Response> {
    processes::resize::resize(&terminal_id, rows, cols)
        .instrument(info_span!("Resize", %terminal_id))
        .await
        .map_err(into_error(StatusCode::INTERNAL_SERVER_ERROR))
}
