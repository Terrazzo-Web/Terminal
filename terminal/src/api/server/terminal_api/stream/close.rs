use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::TerminalAddress;
use crate::backend::client_service::terminal_service;

pub async fn close(
    server: Arc<Server>,
    Json(request): Json<TerminalAddress>,
) -> Result<(), HttpError<self::terminal_service::close::CloseError>> {
    Ok(self::terminal_service::close::close(&server, &request.via, request.id).await?)
}
