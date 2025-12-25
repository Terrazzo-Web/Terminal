use std::sync::Arc;

use terrazzo::axum::Json;
use terrazzo::axum::response::IntoResponse;
use tracing::Instrument as _;
use tracing::info_span;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use self::register::RegisterStreamError;
use crate::api::server::correlation_id::CorrelationId;
use crate::api::shared::terminal_schema::RegisterTerminalRequest;

mod ack;
mod close;
mod pipe;
mod register;
mod registration;

pub use self::ack::ack;
pub use self::close::close;
pub use self::pipe::close_pipe;
pub use self::pipe::keepalive;

pub async fn pipe(server: Arc<Server>, correlation_id: CorrelationId) -> impl IntoResponse {
    pipe::pipe(server, correlation_id)
}

pub async fn register(
    my_client_name: Option<ClientName>,
    server: Arc<Server>,
    Json(request): Json<RegisterTerminalRequest>,
) -> Result<Json<RegisterTerminalRequest>, HttpError<RegisterStreamError>> {
    let span = info_span!("Register", terminal_id = %request.def.address.id);
    let response = register::register(my_client_name, &server, request)
        .instrument(span)
        .await;
    Ok(response?).map(|r| r.into())
}
