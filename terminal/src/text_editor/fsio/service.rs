#![cfg(feature = "server")]

use std::path::PathBuf;
use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use server_fn::ServerFnError;

pub fn load_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<Arc<str>>, ServerFnError> {
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
) -> Result<(), ServerFnError> {
    return Ok(store_file_impl(base_path, file_path, content)?);
}

fn store_file_impl(
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), StoreFileError> {
    let path = PathBuf::from(format!("{base_path}/{file_path}"));
    return if !file_path.is_empty() && path.exists() {
        Ok(std::fs::write(&path, content)?)
    } else {
        Err(StoreFileError::InvalidPath)
    };
}

#[nameth]
#[derive(thiserror::Error, Debug)]
enum StoreFileError {
    #[error("[{n}] {0}", n = self.name())]
    IO(#[from] std::io::Error),

    #[error("[{n}] Invalid path", n = self.name())]
    InvalidPath,
}
