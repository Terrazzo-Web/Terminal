use scopeguard::defer;
use terrazzo::axum::Json;
use tracing::Instrument as _;
use tracing::debug_span;
use tracing::trace;
use trz_gateway_common::http_error::HttpError;

use crate::api::WriteRequest;
use crate::processes;
use crate::processes::write::WriteError;

pub async fn write(Json(request): Json<WriteRequest>) -> Result<(), HttpError<WriteError>> {
    let terminal_id = &request.terminal.id;
    let span = debug_span!("Write", %terminal_id);
    async {
        trace!("Start");
        defer!(trace!("End"));
        Ok(processes::write::write(&terminal_id, request.data.as_bytes()).await?)
    }
    .instrument(span)
    .await
}
