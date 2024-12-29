use terrazzo::axum::Json;
use tracing::info_span;

use crate::api::TerminalDef;
use crate::processes;

pub async fn list() -> Json<Vec<TerminalDef>> {
    let _span = info_span!("List").entered();
    processes::list::list().into()
}
