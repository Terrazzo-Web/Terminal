use std::future::ready;
use std::sync::Arc;

use terrazzo::axum::Json;
use terrazzo::axum::body::Body;
use tracing::Instrument as _;
use tracing::info_span;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use self::register::RegisterStreamError;
use super::correlation_id::CorrelationId;
use crate::api::RegisterTerminalRequest;

mod close;
mod pipe;
mod register;
mod registration;

pub use self::close::close;
pub use self::pipe::close_pipe;

pub fn pipe(correlation_id: CorrelationId) -> impl Future<Output = Body> {
    ready(pipe::pipe(correlation_id))
}

pub async fn register(
    server: Arc<Server>,
    Json(request): Json<RegisterTerminalRequest>,
) -> Result<(), HttpError<RegisterStreamError>> {
    let span = info_span!("Register", terminal_id = %request.def.address.id);
    let response = register::register(&server, request).instrument(span).await;
    Ok(response?)
}
