use scopeguard::defer;
use tracing::Instrument;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::gateway::client::ListTerminalsRequest;
use crate::backend::protos::terrazzo::gateway::client::MaybeString;
use crate::backend::protos::terrazzo::gateway::client::TerminalDef;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;

pub async fn list_terminals(server: &Server, visited: &[String]) -> Vec<TerminalDef> {
    async {
        info!("Start");
        defer!(info!("Done"));
        let mut response = vec![];
        response.extend(processes::list::list().iter().map(|terminal| {
            let title = &terminal.title;
            TerminalDef {
                id: terminal.id.to_string(),
                shell_title: title.shell_title.clone(),
                override_title: title.override_title.clone().map(|s| MaybeString { s }),
                order: terminal.order,
                via: vec![],
            }
        }));
        for client_name in server.connections().clients() {
            async {
                if visited.iter().any(|v| v.as_str() == client_name.as_ref()) {
                    debug!("Already visited");
                    return;
                }
                let Some(client) = server.connections().get_client(&client_name) else {
                    warn!("Client connection not found");
                    return;
                };
                let mut client = ClientServiceClient::new(client);
                let terminals = client.list_terminals(ListTerminalsRequest {
                    visited: visited.to_vec(),
                });
                let Ok(mut terminals) = terminals
                    .await
                    .inspect_err(|error| warn!("Failed: {error}"))
                else {
                    return;
                };
                let mut terminals = std::mem::take(&mut terminals.get_mut().terminals);
                for terminal in &mut terminals {
                    terminal.via.push(client_name.to_string());
                }
                response.extend(terminals);
            }
            .instrument(info_span!("Client terminals", %client_name))
            .await
        }
        debug!("Result = {response:?}");
        return response;
    }
    .instrument(info_span!("List terminals"))
    .await
}
