use std::sync::Arc;

use trz_gateway_common::id::ClientName;
use trz_gateway_server::server::Server;

pub mod convert;
pub mod grpc_error;
pub mod notify_service;
pub mod remote_fn_service;
mod routing;
pub mod shared_service;
pub mod terminal_service;

#[derive(Clone)]
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
