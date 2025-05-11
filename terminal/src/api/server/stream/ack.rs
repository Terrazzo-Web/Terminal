use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::AckRequest;
use crate::backend::client_service::ack;
use crate::backend::client_service::ack::AckError;
use crate::backend::protos::terrazzo::gateway::client::AckRequest as AckRequestProto;

pub async fn ack(
    server: Arc<Server>,
    Json(request): Json<AckRequest>,
) -> Result<(), HttpError<AckError>> {
    let client_address = request.terminal.via.as_slice().to_vec();
    let response = ack::ack(
        &server,
        &client_address,
        AckRequestProto {
            terminal: Some(request.terminal.into()),
            ack: request.ack as u64,
        },
    )
    .await;
    Ok(response?)
}
