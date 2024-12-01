use scopeguard::defer;
use terrazzo::axum::extract::Path;
use terrazzo::axum::response::Response;
use terrazzo::http::StatusCode;
use tracing::info;
use tracing::info_span;

use super::into_error;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn close(Path(terminal_id): Path<TerminalId>) -> Result<(), Response> {
    let _span = info_span!("Close", %terminal_id).entered();
    info!("Start");
    defer!(info!("End"));
    return processes::close::close(&terminal_id).map_err(into_error(StatusCode::BAD_REQUEST));
}
