use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;

use scopeguard::defer;
use tonic::Request;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::shared::Empty;
use crate::backend::protos::terrazzo::terminal::OrderedTerminal;
use crate::backend::protos::terrazzo::terminal::SetOrderRequest;
use crate::backend::protos::terrazzo::terminal::terminal_service_client::TerminalServiceClient;
use crate::processes::get_processes;

pub async fn set_order(server: &Server, terminals: Vec<OrderedTerminal>) {
    async {
        info!("Start");
        defer!(info!("Done"));
        debug!("Terminals = {terminals:?}");

        let mut next: HashMap<String, Vec<OrderedTerminal>> = HashMap::new();
        let processes = get_processes();
        for mut terminal in terminals {
            let address = terminal.address.get_or_insert_default();
            let client_address = address.via.get_or_insert_default();
            let terminal_id = address.terminal_id.as_str().into();
            match client_address.via.as_slice() {
                [] => {
                    let Some(mut entry) = processes.get_mut(&terminal_id) else {
                        warn!("Terminal '{terminal_id}' not found");
                        continue;
                    };
                    entry.0.order = terminal.order;
                }
                [rest @ .., leaf] => {
                    let leaf = leaf.to_owned();
                    client_address.via = rest.to_vec();
                    match next.entry(leaf) {
                        Occupied(mut entry) => {
                            entry.get_mut().push(terminal);
                        }
                        Vacant(entry) => {
                            entry.insert(vec![terminal]);
                        }
                    }
                }
            }
        }

        for (client, terminals) in next {
            let Some(channel) = server.connections().get_client(&client.as_str().into()) else {
                warn!("Client '{client}' not found");
                continue;
            };
            let mut grpc = TerminalServiceClient::new(channel);
            match grpc
                .set_order(Request::new(SetOrderRequest { terminals }))
                .await
            {
                Ok(response) => {
                    let Empty {} = response.into_inner();
                }
                Err(error) => {
                    warn!("Set order on '{client}' failed with {error}");
                }
            }
        }
    }
    .instrument(info_span!("Set order"))
    .await
}
