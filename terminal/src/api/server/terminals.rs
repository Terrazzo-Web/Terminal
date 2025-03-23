use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::backend::client_service::terminals::list_terminals;

pub async fn list(server: Arc<Server>) -> Json<Vec<TerminalDef>> {
    Json(
        list_terminals(&server, &[])
            .await
            .into_iter()
            .map(|terminal_def| TerminalDef {
                id: terminal_def.id.into(),
                title: TabTitle {
                    shell_title: terminal_def.shell_title,
                    override_title: terminal_def.override_title.map(|s| s.s),
                },
                order: terminal_def.order,
                via: terminal_def.via.into_iter().map(ClientName::from).collect(),
            })
            .collect(),
    )
}
