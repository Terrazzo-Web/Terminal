use terrazzo::axum::Json;

use crate::api::TerminalDef;
use crate::processes::next_terminal_id;

pub async fn new_id() -> Json<TerminalDef> {
    let next = next_terminal_id();
    let title = format!("Terminal {next}");
    let id = format!("terminal-{next}").into();
    TerminalDef {
        id,
        title,
        order: next,
    }
    .into()
}
