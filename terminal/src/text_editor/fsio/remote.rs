#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use super::File;
use crate::backend::client_service::grpc_error::GrpcError;
use crate::backend::client_service::remote_fn_service;
use crate::text_editor::file_path::FilePath;

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct LoadFileRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
    pub path: FilePath<Arc<str>>,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct StoreFileRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
    pub path: FilePath<Arc<str>>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub content: String,
}

remote_fn_service::declare_remote_fn!(
    LOAD_FILE_REMOTE_FN,
    super::LOAD_FILE,
    LoadFileRequest,
    Option<File>,
    |_server, arg: LoadFileRequest| {
        let result = super::service::load_file(arg.path);
        ready(result.map_err(GrpcError::from))
    }
);

remote_fn_service::declare_remote_fn!(
    STORE_FILE_REMOTE_FN,
    super::STORE_FILE_IMPL,
    StoreFileRequest,
    (),
    |_server, arg: StoreFileRequest| {
        let result = super::service::store_file(arg.path, arg.content);
        ready(result.map_err(GrpcError::from))
    }
);
