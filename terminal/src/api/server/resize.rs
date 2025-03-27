use terrazzo::axum::Json;
use terrazzo::axum::extract::Path;
use tracing::Instrument as _;
use tracing::info_span;
use trz_gateway_common::http_error::HttpError;

use crate::api::Size;
use crate::processes;
use crate::processes::resize::ResizeOperationError;
use crate::terminal_id::TerminalId;

pub async fn resize(
    Path(terminal_id): Path<TerminalId>,
    Json((Size { rows, cols }, first_resize)): Json<(Size, bool)>,
) -> Result<(), HttpError<ResizeOperationError>> {
    let response = processes::resize::resize(&terminal_id, rows, cols, first_resize)
        .instrument(info_span!("Resize", %terminal_id))
        .await;
    Ok(response?)
}
