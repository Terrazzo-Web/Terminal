use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::WriteRequest;
use crate::backend::client_service::write;
use crate::backend::client_service::write::WriteError;
use crate::backend::protos::terrazzo::gateway::client::WriteRequest as WriteRequestProto;

pub async fn write(
    server: Arc<Server>,
    Json(request): Json<WriteRequest>,
) -> Result<(), HttpError<WriteError>> {
    let client_address = request.terminal.via.as_slice().to_vec();
    Ok(write::write(
        &server,
        &client_address,
        WriteRequestProto {
            terminal: Some(request.terminal.into()),
            data: request.data,
        },
    )
    .await?)
}
