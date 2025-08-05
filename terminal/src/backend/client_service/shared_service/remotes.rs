use std::collections::HashMap;
use std::collections::hash_map;

use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use tracing::warn;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use crate::backend::protos::terrazzo::shared::ClientAddress;
use crate::backend::protos::terrazzo::shared::ListRemotesRequest;
use crate::backend::protos::terrazzo::shared::shared_service_client::SharedServiceClient;

pub fn list_remotes(
    server: &Server,
    visited: Vec<String>,
) -> impl Future<Output = Vec<ClientAddress>> {
    list_remotes_impl(server, visited).instrument(debug_span!("List remotes"))
}

async fn list_remotes_impl(server: &Server, visited: Vec<String>) -> Vec<ClientAddress> {
    debug!("Start");
    defer!(debug!("Done"));
    let mut map = HashMap::new();

    let clients = {
        let mut l = server.connections().clients();
        l.sort();
        l
    };
    let mut next = 0;
    let mut next_entry = move |entry| {
        next += 1;
        (next, entry)
    };
    for client_name in clients {
        if visited.iter().any(|v| v.as_str() == client_name.as_ref()) {
            debug!("Already visited");
            continue;
        }
        map.insert(
            client_name.clone(),
            next_entry(ClientAddress {
                via: vec![client_name.to_string()],
            }),
        );
        list_remotes_for_client(server, &visited, &mut map, &mut next_entry, &client_name)
            .instrument(debug_span!("Client", remote_client_name = %client_name))
            .await
    }

    let mut list = map.into_values().collect::<Vec<_>>();
    list.sort_by_key(|entry| entry.0);
    let response = list.into_iter().map(|entry| entry.1).collect();
    debug!("Result = {response:?}");
    return response;
}

async fn list_remotes_for_client(
    server: &Server,
    visited: &[String],
    map: &mut HashMap<ClientName, (i32, ClientAddress)>,
    next_entry: &mut impl FnMut(ClientAddress) -> (i32, ClientAddress),
    client_name: &ClientName,
) {
    let Some(client) = server.connections().get_client(client_name) else {
        warn!("Client connection not found");
        return;
    };
    let mut client = SharedServiceClient::new(client);
    let remotes = client.list_remotes(ListRemotesRequest {
        visited: visited.to_vec(),
    });
    let Ok(mut remotes) = remotes.await.inspect_err(|error| warn!("Failed: {error}")) else {
        return;
    };
    let remotes = std::mem::take(&mut remotes.get_mut().clients);
    for mut remote in remotes {
        let Some(remote_name) = remote.leaf() else {
            continue;
        };
        match map.entry(remote_name) {
            hash_map::Entry::Occupied(mut entry) => {
                if entry.get().1.via.len() > remote.via.len() + 1 {
                    remote.via.push(client_name.to_string());
                    entry.insert(next_entry(remote));
                }
            }
            hash_map::Entry::Vacant(entry) => {
                remote.via.push(client_name.to_string());
                entry.insert(next_entry(remote));
            }
        }
    }
}
