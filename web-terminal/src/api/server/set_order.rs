use scopeguard::defer;
use terrazzo::axum::response::Response;
use terrazzo::axum::Json;
use tracing::debug_span;
use tracing::trace;

use crate::processes;
use crate::terminal_id::TerminalId;

pub async fn set_order(Json(ids): Json<Vec<TerminalId>>) -> Result<(), Response> {
    let span = debug_span!("SetOrder");
    span.in_scope(|| trace!("Start"));
    defer!(span.in_scope(|| trace!("End")));
    processes::set_order::set_order(ids);
    Ok(())
}
