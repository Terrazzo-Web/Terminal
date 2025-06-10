#![cfg(feature = "server")]

use std::path::PathBuf;
use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tonic::Code;

use crate::backend::client_service::grpc_error::IsGrpcError;

pub fn load_file(base_path: Arc<str>, file_path: Arc<str>) -> Result<Option<Arc<str>>, FsioError> {
    let path = PathBuf::from(format!("{base_path}/{file_path}"));
    if !file_path.is_empty() && path.exists() {
        Ok(Some(Arc::from(std::fs::read_to_string(&path)?)))
    } else {
        Ok(None)
    }
}

pub fn store_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), FsioError> {
    let path = PathBuf::from(format!("{base_path}/{file_path}"));
    return if !file_path.is_empty() && path.exists() {
        Ok(std::fs::write(&path, content)?)
    } else {
        Err(FsioError::InvalidPath)
    };
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum FsioError {
    #[error("[{n}] {0}", n = self.name())]
    IO(#[from] std::io::Error),

    #[error("[{n}] Invalid path", n = self.name())]
    InvalidPath,
}

impl IsGrpcError for FsioError {
    fn code(&self) -> Code {
        match self {
            Self::IO { .. } => Code::FailedPrecondition,
            Self::InvalidPath => Code::InvalidArgument,
        }
    }
}
