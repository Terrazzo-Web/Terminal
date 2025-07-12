use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::ResizeRequest;
use crate::backend::client_service::terminal_service;
use crate::backend::protos::terrazzo::terminal::ResizeRequest as ResizeRequestProto;
use crate::backend::protos::terrazzo::terminal::Size as SizeProto;

pub async fn resize(
    server: Arc<Server>,
    Json(request): Json<ResizeRequest>,
) -> Result<(), HttpError<self::terminal_service::resize::ResizeError>> {
    let client_address = request.terminal.via.to_vec();
    let response = self::terminal_service::resize::resize(
        &server,
        &client_address,
        ResizeRequestProto {
            terminal: Some(request.terminal.into()),
            size: Some(SizeProto {
                rows: request.size.rows,
                cols: request.size.cols,
            }),
            force: request.force,
        },
    )
    .await;
    Ok(response?)
}
