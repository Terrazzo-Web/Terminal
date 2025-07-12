use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::WriteRequest;
use crate::backend::client_service::terminal_service;
use crate::backend::protos::terrazzo::terminal::WriteRequest as WriteRequestProto;

pub async fn write(
    server: Arc<Server>,
    Json(request): Json<WriteRequest>,
) -> Result<(), HttpError<self::terminal_service::write::WriteError>> {
    let client_address = request.terminal.via.to_vec();
    Ok(self::terminal_service::write::write(
        &server,
        &client_address,
        WriteRequestProto {
            terminal: Some(request.terminal.into()),
            data: request.data,
        },
    )
    .await?)
}
