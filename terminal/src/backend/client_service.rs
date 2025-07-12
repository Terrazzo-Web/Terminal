use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;
use tonic::async_trait;
use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

use self::notify::RemoteResponseStream;
use self::notify::notify_hybrid;
use self::remotes::list_remotes;
use super::protos::terrazzo::gateway::client::AckRequest;
use super::protos::terrazzo::gateway::client::Empty;
use super::protos::terrazzo::gateway::client::ListRemotesRequest;
use super::protos::terrazzo::gateway::client::ListRemotesResponse;
use super::protos::terrazzo::gateway::client::ListTerminalsRequest;
use super::protos::terrazzo::gateway::client::ListTerminalsResponse;
use super::protos::terrazzo::gateway::client::NewIdRequest;
use super::protos::terrazzo::gateway::client::NewIdResponse;
use super::protos::terrazzo::gateway::client::RegisterTerminalRequest;
use super::protos::terrazzo::gateway::client::ResizeRequest;
use super::protos::terrazzo::gateway::client::SetOrderRequest;
use super::protos::terrazzo::gateway::client::SetTitleRequest;
use super::protos::terrazzo::gateway::client::TerminalAddress;
use super::protos::terrazzo::gateway::client::WriteRequest;
use super::protos::terrazzo::gateway::client::client_service_server::ClientService;
use crate::backend::protos::terrazzo::gateway::client::NotifyRequest;
use crate::backend::protos::terrazzo::gateway::client::RemoteFnRequest;
use crate::backend::protos::terrazzo::gateway::client::ServerFnResponse;
use crate::processes::io::RemoteReader;

pub mod convert;
pub mod grpc_error;
pub mod notify;
pub mod remote_fn;
pub mod remotes;
mod routing;
pub mod terminal_service;

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
        request: Request<ListRemotesRequest>,
    ) -> Result<Response<ListRemotesResponse>, Status> {
        let mut visited = request.into_inner().visited;
        visited.push(self.client_name.to_string());
        let clients = list_remotes(&self.server, visited).await;
        Ok(Response::new(ListRemotesResponse { clients }))
    }

    async fn list_terminals(
        &self,
        mut request: Request<ListTerminalsRequest>,
    ) -> Result<Response<ListTerminalsResponse>, Status> {
        use self::terminal_service::terminals::list_terminals;
        let mut visited = std::mem::take(&mut request.get_mut().visited);
        visited.push(self.client_name.to_string());
        let terminals = list_terminals(&self.server, visited).await;
        Ok(Response::new(ListTerminalsResponse { terminals }))
    }

    async fn new_id(
        &self,
        request: Request<NewIdRequest>,
    ) -> Result<Response<NewIdResponse>, Status> {
        use self::terminal_service::new_id::new_id;
        let address = request.into_inner().address;
        let next = new_id(
            &self.server,
            address.as_ref().map(|a| a.via.as_slice()).unwrap_or(&[]),
        )
        .await?;
        Ok(Response::new(NewIdResponse { next }))
    }

    type RegisterStream = RemoteReader;

    async fn register(
        &self,
        request: Request<RegisterTerminalRequest>,
    ) -> Result<Response<Self::RegisterStream>, Status> {
        use self::terminal_service::register::register;
        let stream = register(
            Some(self.client_name.clone()),
            &self.server,
            request.into_inner(),
        )
        .await?;
        Ok(Response::new(RemoteReader(stream)))
    }

    async fn write(&self, request: Request<WriteRequest>) -> Result<Response<Empty>, Status> {
        use self::terminal_service::write::write;
        let mut request = request.into_inner();
        let terminal = request.terminal.get_or_insert_default();
        let client_address = std::mem::take(&mut terminal.via.get_or_insert_default().via);
        let () = write(&self.server, &client_address, request).await?;
        Ok(Response::new(Empty {}))
    }

    async fn resize(&self, request: Request<ResizeRequest>) -> Result<Response<Empty>, Status> {
        use self::terminal_service::resize::resize;
        let mut request = request.into_inner();
        let terminal = request.terminal.get_or_insert_default();
        let client_address = std::mem::take(&mut terminal.via.get_or_insert_default().via);
        let () = resize(&self.server, &client_address, request).await?;
        Ok(Response::new(Empty {}))
    }

    async fn close(&self, request: Request<TerminalAddress>) -> Result<Response<Empty>, Status> {
        use self::terminal_service::close::close;
        let terminal = request.into_inner();
        let terminal_id = terminal.terminal_id.as_str().into();
        let client_address = terminal.client_address();
        let () = close(&self.server, client_address, terminal_id).await?;
        Ok(Response::new(Empty {}))
    }

    async fn set_title(
        &self,
        request: Request<SetTitleRequest>,
    ) -> Result<Response<Empty>, Status> {
        use self::terminal_service::set_title::set_title;
        let mut request = request.into_inner();
        let terminal = request.address.get_or_insert_default();
        let client_address = std::mem::take(&mut terminal.via.get_or_insert_default().via);
        let () = set_title(&self.server, &client_address, request).await?;
        Ok(Response::new(Empty {}))
    }

    async fn set_order(
        &self,
        request: Request<SetOrderRequest>,
    ) -> Result<Response<Empty>, Status> {
        use self::terminal_service::set_order::set_order;
        let () = set_order(&self.server, request.into_inner().terminals).await;
        Ok(Response::new(Empty {}))
    }

    async fn ack(&self, request: Request<AckRequest>) -> Result<Response<Empty>, Status> {
        use self::terminal_service::ack::ack;
        let mut request = request.into_inner();
        let terminal = request.terminal.get_or_insert_default();
        let client_address = std::mem::take(&mut terminal.via.get_or_insert_default().via);
        let () = ack(&self.server, &client_address, request).await?;
        Ok(Response::new(Empty {}))
    }

    async fn call_server_fn(
        &self,
        request: Request<RemoteFnRequest>,
    ) -> Result<Response<ServerFnResponse>, Status> {
        let mut request = request.into_inner();
        let address = request.address.get_or_insert_default();
        let address = std::mem::take(&mut address.via);
        let response = remote_fn::dispatch(&self.server, &address, request).await;
        Ok(Response::new(ServerFnResponse { json: response? }))
    }

    type NotifyStream = RemoteResponseStream;

    async fn notify(
        &self,
        request: Request<Streaming<NotifyRequest>>,
    ) -> Result<Response<Self::NotifyStream>, Status> {
        notify_hybrid(request.into_inner().into())
            .map(|response| RemoteResponseStream(response).into())
            .map_err(Status::from)
    }
}
