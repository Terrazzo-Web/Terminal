#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_server::server::Server;

use crate::backend::client_service::grpc_error::GrpcError;
use crate::backend::client_service::remote_fn;
use crate::backend::client_service::remote_fn::RemoteFn;
use crate::backend::client_service::remote_fn::RemoteFnResult;

pub static LOAD_FILE_REMOTE_FN: RemoteFn = RemoteFn {
    name: super::LOAD_FILE,
    callback: load_file,
};

inventory::submit! { LOAD_FILE_REMOTE_FN }

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadFileRequest {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
}

fn load_file(server: &Server, arg: &str) -> RemoteFnResult {
    let load_file = remote_fn::uplift(|_server, arg: LoadFileRequest| {
        ready(super::service::load_file(arg.base_path, arg.file_path).map_err(GrpcError::from))
    });
    Box::pin(load_file(server, arg))
}

pub static STORE_FILE_REMOTE_FN: RemoteFn = RemoteFn {
    name: super::STORE_FILE_IMPL,
    callback: store_file,
};

inventory::submit! { STORE_FILE_REMOTE_FN }

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreFileRequest {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
    pub content: String,
}

fn store_file(server: &Server, arg: &str) -> RemoteFnResult {
    let store_file = remote_fn::uplift(|_server, arg: StoreFileRequest| {
        ready(
            super::service::store_file(arg.base_path, arg.file_path, arg.content)
                .map_err(GrpcError::from),
        )
    });
    Box::pin(store_file(server, arg))
}
