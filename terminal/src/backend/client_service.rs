use std::sync::Arc;

use new_id::NewIdError;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use self::remotes::list_remotes;
use self::terminals::list_terminals;
use super::protos::terrazzo::gateway::client::ListRemotesRequest;
use super::protos::terrazzo::gateway::client::ListRemotesResponse;
use super::protos::terrazzo::gateway::client::ListTerminalsRequest;
use super::protos::terrazzo::gateway::client::ListTerminalsResponse;
use super::protos::terrazzo::gateway::client::NewIdRequest;
use super::protos::terrazzo::gateway::client::NewIdResponse;
use super::protos::terrazzo::gateway::client::RegisterTerminalRequest;
use super::protos::terrazzo::gateway::client::client_service_server::ClientService;
use crate::processes::io::RemoteReader;

pub mod new_id;
pub mod register;
pub mod remotes;
mod routing;
pub mod terminals;

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
    async fn list_remotes(
        &self,
        mut request: Request<ListRemotesRequest>,
    ) -> Result<Response<ListRemotesResponse>, Status> {
        let mut visited = std::mem::take(&mut request.get_mut().visited);
        visited.push(self.client_name.to_string());
        let clients = list_remotes(&self.server, &visited).await;
        Ok(Response::new(ListRemotesResponse { clients }))
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

    async fn new_id(
        &self,
        request: Request<NewIdRequest>,
    ) -> Result<Response<NewIdResponse>, Status> {
        let next = new_id::new_id(
            &self.server,
            request
                .get_ref()
                .address
                .as_ref()
                .map(|a| a.via.as_slice())
                .unwrap_or(&[]),
        )
        .await
        .map_err(|NewIdError::NewIdError(error)| match error {
            routing::DistributedCallbackError::RemoteError(status) => status,
            error @ routing::DistributedCallbackError::RemoteClientNotFound { .. } => {
                Status::not_found(error.to_string())
            }
        })?;
        Ok(Response::new(NewIdResponse { next }))
    }

    type RegisterStream = RemoteReader;

    async fn register(
        &self,
        request: Request<RegisterTerminalRequest>,
    ) -> Result<Response<Self::RegisterStream>, Status> {
        let stream = register::register(&self.server, request.into_inner()).await?;
        Ok(Response::new(RemoteReader(stream)))
    }
}
