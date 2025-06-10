use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::TerminalAddress;
use crate::backend::client_service::close;
use crate::backend::client_service::close::CloseError;

pub async fn close(
    server: Arc<Server>,
    Json(request): Json<TerminalAddress>,
) -> Result<(), HttpError<CloseError>> {
    Ok(close::close(&server, &request.via, request.id).await?)
}
