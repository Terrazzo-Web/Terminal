use std::future::ready;

use terrazzo::axum::body::Body;
use terrazzo::axum::response::Response;
use terrazzo::axum::Json;
use terrazzo::http::StatusCode;
use tracing::info_span;
use tracing::Instrument;

use super::correlation_id::CorrelationId;
use super::into_error;
use crate::api::RegisterTerminalRequest;

mod close;
mod pipe;
mod register;
mod registration;

pub use self::close::close;
pub use self::pipe::close_pipe;

pub fn pipe(correlation_id: CorrelationId) -> impl std::future::Future<Output = Body> {
    ready(pipe::pipe(correlation_id))
}

pub async fn register(Json(request): Json<RegisterTerminalRequest>) -> Result<(), Response> {
    let span = info_span!("Register", terminal_id = %request.def.id);
    register::register(request)
        .instrument(span)
        .await
        .map_err(into_error(StatusCode::BAD_REQUEST))
}
