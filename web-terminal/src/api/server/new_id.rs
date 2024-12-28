use terrazzo::axum::Json;
use uuid::Uuid;

use crate::api::TerminalDef;
use crate::processes::next_terminal_id;

pub async fn new_id() -> Json<TerminalDef> {
    let next = next_terminal_id();
    let title = format!("Terminal {next}");
    let id = Uuid::new_v4().to_string().into();
    TerminalDef {
        id,
        title,
        order: next,
    }
    .into()
}
