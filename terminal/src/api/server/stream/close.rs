use scopeguard::defer;
use terrazzo::axum::extract::Path;
use tracing::info;
use tracing::info_span;
use trz_gateway_common::http_error::HttpError;

use crate::processes;
use crate::processes::close::CloseProcessError;
use crate::terminal_id::TerminalId;

pub async fn close(
    Path(terminal_id): Path<TerminalId>,
) -> Result<(), HttpError<CloseProcessError>> {
    let _span = info_span!("Close", %terminal_id).entered();
    info!("Start");
    defer!(info!("End"));
    return Ok(processes::close::close(&terminal_id)?);
}
