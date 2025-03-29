use std::sync::Arc;

use terrazzo::axum::Json;
use tracing::Instrument;
use tracing::debug_span;
use trz_gateway_server::server::Server;

use crate::api::TerminalAddress;
use crate::backend::client_service::set_order;
use crate::backend::protos::terrazzo::gateway::client::OrderedTerminal;

pub async fn set_order(server: Arc<Server>, Json(terminals): Json<Vec<TerminalAddress>>) {
    let () = set_order::set_order(
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
