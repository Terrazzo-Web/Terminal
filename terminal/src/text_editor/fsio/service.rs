#![cfg(feature = "server")]

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tonic::Code;
use tracing::debug;

use crate::backend::client_service::grpc_error::IsGrpcError;
use crate::text_editor::fsio::File;
use crate::text_editor::fsio::FileMetadata;

const MAX_FILES_SORTED: usize = 5000;
const MAX_FILES_RETURNED: usize = 1000;

pub fn load_file(base_path: Arc<str>, file_path: Arc<str>) -> Result<Option<File>, FsioError> {
    let path = PathBuf::from(format!("{base_path}/{file_path}"));
    if !file_path.is_empty() {
        if let Ok(metadata) = path.metadata() {
            if metadata.is_file() {
                debug!("Loading file {path:?}");
                let data = std::fs::read_to_string(&path)?;
                return Ok(Some(File::TextFile {
                    metadata: FileMetadata::single(&path, &metadata).into(),
                    content: Arc::from(data),
                }));
            }
            if metadata.is_dir() {
                debug!("Loading file {path:?}");
                let mut files = vec![];
                let mut uids = HashMap::default();
                let mut gids = HashMap::default();
                for file in path
                    .read_dir()?
                    .filter_map(|f| f.ok())
                    .take(MAX_FILES_SORTED)
                {
                    files.push(FileMetadata::of(file, &mut uids, &mut gids));
                }
                files.sort_by_key(|f| Reverse(f.modified));
                let mut files = files
                    .into_iter()
                    .take(MAX_FILES_RETURNED)
                    .collect::<Vec<_>>();
                files.sort_by(|a, b| Ord::cmp(&a.name, &b.name));
                return Ok(Some(File::Folder(Arc::from(files))));
            }
        }
    }
    debug!("Not found {path:?}");
    Ok(None)
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

#[cfg(test)]
#[test]
fn check_option_order() {
    assert!(None < Some(-2));
    assert!(Some(1) < Some(2));
}
