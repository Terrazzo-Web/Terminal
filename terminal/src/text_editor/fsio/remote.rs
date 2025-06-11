#![cfg(feature = "server")]

use std::sync::Arc;

use crate::backend::client_service::remote_fn;

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct LoadFileRequest {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
}

#[derive(Debug, serde::Serialize, serde:: Deserialize)]
pub struct StoreFileRequest {
    pub base_path: Arc<str>,
    pub file_path: Arc<str>,
    pub content: String,
}

remote_fn::declare_remote_fn! {
    LOAD_FILE_REMOTE_FN,
    LOAD_FILE,
    |_server, arg: LoadFileRequest| {
        use crate::backend::client_service::grpc_error::GrpcError;
        use std::future::ready;
        let result = super::service::load_file(arg.base_path, arg.file_path);
        ready(result.map_err(GrpcError::from))
    }
}

remote_fn::declare_remote_fn! {
    STORE_FILE_REMOTE_FN,
    STORE_FILE_IMPL,
    |_server, arg: StoreFileRequest| {
        use crate::backend::client_service::grpc_error::GrpcError;
        use std::future::ready;
        let result = service::store_file(arg.base_path, arg.file_path, arg.content);
        ready(result.map_err(GrpcError::from))
    }
}
