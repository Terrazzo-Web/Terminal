use std::sync::Arc;

use axum::http::Request;
use terrazzo::axum;
use terrazzo::axum::body::Body;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use super::manager::add_upload_stream;
use crate::api::server::correlation_id::CorrelationId;

pub async fn upload(
    _my_client_name: Option<ClientName>,
    _server: Arc<Server>,
    correlation_id: CorrelationId,
    request: Request<Body>,
) -> String {
    let upload_stream = request.into_body().into_data_stream();
    let () = add_upload_stream(correlation_id, upload_stream).await;
    String::default()
}
