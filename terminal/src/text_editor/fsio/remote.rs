#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use crate::backend::client_service::grpc_error::GrpcError;
use crate::backend::client_service::remote_fn;

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct LoadFileRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    pub base_path: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file_path: Arc<str>,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct StoreFileRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    pub base_path: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file_path: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub content: String,
}

remote_fn::declare_remote_fn!(
    LOAD_FILE_REMOTE_FN,
    super::LOAD_FILE,
    |_server, arg: LoadFileRequest| {
        let result = super::service::load_file(arg.base_path, arg.file_path);
        ready(result.map_err(GrpcError::from))
    }
);

remote_fn::declare_remote_fn!(
    STORE_FILE_REMOTE_FN,
    super::STORE_FILE_IMPL,
    |_server, arg: StoreFileRequest| {
        let result = super::service::store_file(arg.base_path, arg.file_path, arg.content);
        ready(result.map_err(GrpcError::from))
    }
);
