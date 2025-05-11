use std::sync::Arc;

use axum::Json;
use axum::response::IntoResponse;
use terrazzo::axum;
use tracing::Instrument as _;
use tracing::info_span;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use self::register::RegisterStreamError;
use super::correlation_id::CorrelationId;
use crate::api::RegisterTerminalRequest;

mod ack;
mod close;
mod pipe;
mod register;
mod registration;

pub use self::ack::ack;
pub use self::close::close;
pub use self::pipe::close_pipe;
pub use self::pipe::keepalive;

pub async fn pipe(correlation_id: CorrelationId) -> impl IntoResponse {
    pipe::pipe(correlation_id)
}

pub async fn register(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    Json(request): Json<RegisterTerminalRequest>,
) -> Result<(), HttpError<RegisterStreamError>> {
    let span = info_span!("Register", terminal_id = %request.def.address.id);
    let response = register::register(my_client_name, &server, request)
        .instrument(span)
        .await;
    Ok(response?)
}
