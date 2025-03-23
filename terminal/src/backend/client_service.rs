use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;
use tracing::info;
use tracing::warn;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use super::protos::terrazzo::gateway::client::ListTerminalsRequest;
use super::protos::terrazzo::gateway::client::ListTerminalsResponse;
use super::protos::terrazzo::gateway::client::MaybeString;
use super::protos::terrazzo::gateway::client::NewIdRequest;
use super::protos::terrazzo::gateway::client::NewIdResponse;
use super::protos::terrazzo::gateway::client::TerminalDef;
use super::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use super::protos::terrazzo::gateway::client::client_service_server::ClientService;
use crate::processes;
use crate::processes::next_terminal_id;

pub struct ClientServiceImpl {
    client_name: ClientName,
    server: Arc<Server>,
}

impl ClientServiceImpl {
    pub fn new(client_name: ClientName, server: Arc<Server>) -> Self {
        Self {
            client_name,
            server,
        }
    }
}

#[async_trait]
impl ClientService for ClientServiceImpl {
    async fn new_id(&self, _: Request<NewIdRequest>) -> Result<Response<NewIdResponse>, Status> {
        let next = next_terminal_id();
        info!("Allocate new ID: {next}");
        Ok(Response::new(NewIdResponse { next }))
    }

    async fn list_terminals(
        &self,
        mut request: Request<ListTerminalsRequest>,
    ) -> Result<Response<ListTerminalsResponse>, Status> {
        let mut visited = std::mem::take(&mut request.get_mut().visited);
        visited.push(self.client_name.to_string());
        let terminals = list_terminals(&self.server, &visited).await;
        Ok(Response::new(ListTerminalsResponse { terminals }))
    }
}

pub async fn list_terminals(server: &Server, visited: &[String]) -> Vec<TerminalDef> {
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
        if visited.iter().any(|v| v.as_str() == client_name.as_ref()) {
            continue;
        }
        let Some(client) = server.connections().get_client(&client_name) else {
            continue;
        };
        let mut client = ClientServiceClient::new(client);
        let Ok(mut terminals) = client
            .list_terminals(ListTerminalsRequest {
                visited: visited.to_vec(),
            })
            .await
            .inspect_err(|error| warn!("List terminals failed for {client_name}: {error}"))
        else {
            continue;
        };
        let mut terminals = std::mem::take(&mut terminals.get_mut().terminals);
        for terminal in &mut terminals {
            terminal.via.push(client_name.to_string());
        }
        response.extend(terminals);
    }
    return response;
}
