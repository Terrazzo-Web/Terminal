use axum::extract::Path;
use axum::response::Response;
use http::StatusCode;
use scopeguard::defer;
use tracing::debug_span;
use tracing::trace;

use super::into_error;
use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn set_title(
    Path(terminal_id): Path<TerminalId>,
    new_title: String,
) -> Result<(), Response> {
    let span = debug_span!("SetTitle", %terminal_id);
    span.in_scope(|| trace!("Start"));
    defer!(span.in_scope(|| trace!("End")));
    processes::set_title::set_title(&terminal_id, new_title)
        .map_err(into_error(StatusCode::BAD_REQUEST))
}
