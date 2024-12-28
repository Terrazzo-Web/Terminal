use axum::extract::Path;
use axum::response::Response;
use http::StatusCode;
use scopeguard::defer;
use tracing::info;
use tracing::info_span;

use super::into_error;
use crate::api::server::correlation_id::CorrelationId;
use crate::api::server::stream::registration::Registration;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn close(
    Path(terminal_id): Path<TerminalId>,
    correlation_id: Option<CorrelationId>,
) -> Result<(), Response> {
    let _span = info_span!("Close", %terminal_id).entered();
    info!("Start");
    defer!(info!("End"));
    if let Some(correlation_id) = correlation_id {
        drop(Registration::get_if(&correlation_id));
    }
    return processes::close::close(&terminal_id).map_err(into_error(StatusCode::BAD_REQUEST));
}
