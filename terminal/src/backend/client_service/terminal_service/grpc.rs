use tonic::Request;
use tonic::Response;
use tonic::Result;
use tonic::Status;
use tonic::async_trait;

use crate::backend::client_service::ClientServiceImpl;
use crate::backend::client_service::terminal_service;
use crate::backend::protos::terrazzo::shared::Empty;
use crate::backend::protos::terrazzo::terminal::AckRequest;
use crate::backend::protos::terrazzo::terminal::ListTerminalsRequest;
use crate::backend::protos::terrazzo::terminal::ListTerminalsResponse;
use crate::backend::protos::terrazzo::terminal::NewIdRequest;
use crate::backend::protos::terrazzo::terminal::NewIdResponse;
use crate::backend::protos::terrazzo::terminal::RegisterTerminalRequest;
use crate::backend::protos::terrazzo::terminal::ResizeRequest;
use crate::backend::protos::terrazzo::terminal::SetOrderRequest;
use crate::backend::protos::terrazzo::terminal::SetTitleRequest;
use crate::backend::protos::terrazzo::terminal::TerminalAddress;
use crate::backend::protos::terrazzo::terminal::WriteRequest;
use crate::backend::protos::terrazzo::terminal::terminal_service_server::TerminalService;
use crate::processes::io::RemoteReader;

#[async_trait]
impl TerminalService for ClientServiceImpl {
    async fn list_terminals(
        &self,
        mut request: Request<ListTerminalsRequest>,
    ) -> Result<Response<ListTerminalsResponse>, Status> {
        use self::terminal_service::list::list_terminals;
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
}
