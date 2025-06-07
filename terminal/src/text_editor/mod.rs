use std::sync::Arc;

use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

mod service;
mod state;
pub mod ui;

#[server]
async fn autocomplete_path(
    kind: PathSelector,
    prefix: Arc<str>,
    path: Arc<str>,
) -> Result<Vec<String>, ServerFnError> {
    Ok(service::autocomplete_path(kind, &prefix, &path)?)
}

#[server]
async fn load_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<Arc<str>>, ServerFnError> {
    use std::path::PathBuf;
    let path = PathBuf::from(format!("{base_path}/{file_path}"));
    if !file_path.is_empty() && path.exists() {
        Ok(Some(Arc::from(std::fs::read_to_string(&path)?)))
    } else {
        Ok(None)
    }
}

#[server]
async fn store_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), ServerFnError> {
    return Ok(aux(base_path, file_path, content)?);

    use std::path::PathBuf;

    use nameth::NamedEnumValues as _;

    fn aux(
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
}

#[nameth]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
enum PathSelector {
    BasePath,
    FilePath,
}

#[cfg(feature = "client")]
mod path_selector_client {
    use super::PathSelector;
    use crate::assets::icons;

    impl PathSelector {
        pub fn icon(self) -> icons::Icon {
            match self {
                Self::BasePath => icons::slash(),
                Self::FilePath => icons::chevron_double_right(),
            }
        }
    }
}

#[cfg(feature = "server")]
mod path_selector_server {
    use std::fs::Metadata;

    use super::PathSelector;

    impl PathSelector {
        pub fn accept(self, metadata: &Metadata) -> bool {
            match self {
                Self::BasePath => metadata.is_dir(),
                Self::FilePath => true,
            }
        }
    }
}
