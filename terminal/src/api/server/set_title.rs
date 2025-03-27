use scopeguard::defer;
use terrazzo::axum::Json;
use terrazzo::axum::extract::Path;
use tracing::debug_span;
use tracing::trace;
use trz_gateway_common::http_error::HttpError;

use crate::api::TabTitle;
use crate::processes;
use crate::processes::set_title::SetTitleError;
use crate::terminal_id::TerminalId;

pub async fn set_title(
    Path(terminal_id): Path<TerminalId>,
    Json(new_title): Json<TabTitle<String>>,
) -> Result<(), HttpError<SetTitleError>> {
    let _span = debug_span!("SetTitle", %terminal_id).entered();
    trace!("Start");
    defer!(trace!("End"));
    Ok(processes::set_title::set_title(&terminal_id, new_title)?)
}
