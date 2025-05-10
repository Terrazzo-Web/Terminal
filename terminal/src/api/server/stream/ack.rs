use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_server::server::Server;

use crate::api::AckRequest;
use crate::backend::throttling_stream;

pub async fn ack(_server: Arc<Server>, Json(request): Json<AckRequest>) {
    throttling_stream::ack(&request.terminal.id, request.ack)
}
