use terrazzo::axum::extract::Path;
use terrazzo::axum::response::Response;
use terrazzo::axum::Json;
use terrazzo::http::StatusCode;
use tracing::info_span;
use tracing::Instrument;

use super::into_error;
use crate::api::Size;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn resize(
    Path(terminal_id): Path<TerminalId>,
    Json((Size { rows, cols }, first_resize)): Json<(Size, bool)>,
) -> Result<(), Response> {
    processes::resize::resize(&terminal_id, rows, cols, first_resize)
        .instrument(info_span!("Resize", %terminal_id))
        .await
        .map_err(into_error(StatusCode::INTERNAL_SERVER_ERROR))
}
