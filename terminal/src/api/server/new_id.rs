use terrazzo::axum::extract::Json;
use uuid::Uuid;

use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::api::client_name::ClientName;
use crate::processes::next_terminal_id;

pub async fn new_id(Json(client_name): Json<Option<ClientName>>) -> Json<TerminalDef> {
    let next = next_terminal_id();
    let title = format!("Terminal {next}");
    let id = if cfg!(feature = "concise_traces") {
        Uuid::new_v4().to_string().into()
    } else {
        format!("T-{next}").into()
    };
    TerminalDef {
        id,
        title: TabTitle {
            shell_title: title,
            override_title: None,
        },
        order: next,
        client_name,
    }
    .into()
}
