use std::sync::Arc;

use terrazzo::axum::Json;
use tracing::Instrument as _;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use crate::api::shared::terminal_schema::TerminalAddress;
use crate::backend::client_service::terminal_service;
use crate::backend::protos::terrazzo::terminal::OrderedTerminal;

pub async fn set_order(server: Arc<Server>, Json(terminals): Json<Vec<TerminalAddress>>) {
    let () = self::terminal_service::set_order::set_order(
        &server,
        terminals
            .into_iter()
            .enumerate()
            .map(|(order, terminal)| OrderedTerminal {
                address: Some(terminal.into()),
                order: order as i32,
            })
            .collect(),
    )
    .instrument(debug_span!("SetOrder"))
    .await;
}
