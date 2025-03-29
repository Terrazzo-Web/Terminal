use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::ResizeRequest;
use crate::backend::client_service::resize;
use crate::backend::client_service::resize::ResizeError;
use crate::backend::protos::terrazzo::gateway::client::ResizeRequest as ResizeRequestProto;
use crate::backend::protos::terrazzo::gateway::client::Size;

pub async fn resize(
    server: Arc<Server>,
    Json(request): Json<ResizeRequest>,
) -> Result<(), HttpError<ResizeError>> {
    let client_address = request.terminal.via.as_slice().to_vec();
    let response = resize::resize(
        &server,
        &client_address,
        ResizeRequestProto {
            terminal: Some(request.terminal.into()),
            size: Some(Size {
                rows: request.size.rows,
                cols: request.size.cols,
            }),
            force: request.force,
        },
    )
    .await;
    Ok(response?)
}
