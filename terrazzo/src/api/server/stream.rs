use axum::body::Body;
use axum::extract::Path;
use axum::response::Response;
use http::StatusCode;
use tracing::info_span;
use tracing::Instrument;

use super::correlation_id::CorrelationId;
use super::into_error;
use crate::terminal_id::TerminalId;

mod close;
mod pipe;
mod register;
mod registration;

pub fn pipe(correlation_id: CorrelationId) -> impl std::future::Future<Output = Body> {
    pipe::pipe(correlation_id).instrument(info_span!("Pipe"))
}

pub async fn register(Path(terminal_id): Path<TerminalId>) -> Result<(), Response> {
    let span = info_span!("Register", %terminal_id);
    register::register(terminal_id)
        .instrument(span)
        .await
        .map_err(into_error(StatusCode::BAD_REQUEST))
}

pub use self::close::close;
