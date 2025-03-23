use std::collections::HashMap;
use std::collections::hash_map;

use scopeguard::defer;
use tracing::Instrument;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ListRemotesRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;

pub async fn list_remotes(server: &Server, visited: &[String]) -> Vec<ClientAddress> {
    async {
        info!("Start");
        defer!(info!("Done"));
        let mut map = HashMap::new();

        for client_name in server.connections().clients() {
            map.insert(
                client_name.clone(),
                ClientAddress {
                    via: vec![client_name.to_string()],
                },
            );
            let task = async {
                if visited.iter().any(|v| v.as_str() == client_name.as_ref()) {
                    debug!("Already visited");
                    return;
                }
                let Some(client) = server.connections().get_client(&client_name) else {
                    warn!("Client connection not found");
                    return;
                };
                let mut client = ClientServiceClient::new(client);
                let Ok(mut remotes) = client
                    .list_remotes(ListRemotesRequest {
                        visited: visited.to_vec(),
                    })
                    .await
                    .inspect_err(|error| warn!("Failed: {error}"))
                else {
                    return;
                };
                let remotes = std::mem::take(&mut remotes.get_mut().clients);
                for mut remote in remotes {
                    match map.entry(remote.leaf()) {
                        hash_map::Entry::Occupied(mut entry) => {
                            if entry.get().via.len() >= remote.via.len() + 1 {
                                remote.via.push(client_name.to_string());
                                entry.insert(remote);
                            }
                        }
                        hash_map::Entry::Vacant(entry) => {
                            remote.via.push(client_name.to_string());
                            entry.insert(remote);
                        }
                    }
                }
            }
            .instrument(info_span!("Client", %client_name));
            check_sync(&task);
            task.await;
        }

        let response = map.into_values().collect();
        debug!("Result = {response:?}");
        return response;
    }
    .instrument(info_span!("List remotes"))
    .await
}

#[allow(unused)]
fn check_send<T: Send>(t: &T) {}

#[allow(unused)]
fn check_sync<T: Sync>(t: &T) {}
