use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::AckRequest;
use crate::backend::client_service::terminal_service;
use crate::backend::protos::terrazzo::gateway::client::AckRequest as AckRequestProto;

pub async fn ack(
    server: Arc<Server>,
    Json(request): Json<AckRequest>,
) -> Result<(), HttpError<self::terminal_service::ack::AckError>> {
    let client_address = request.terminal.via.to_vec();
    let response = self::terminal_service::ack::ack(
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
