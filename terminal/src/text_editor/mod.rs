use nameth::nameth;
use server_fn::ServerFnError;
use terrazzo::server;

mod text_editor_service;
pub mod text_editor_ui;

#[server]
pub async fn autocomplete_path(
    kind: PathSelector,
    prefix: String,
    path: String,
) -> Result<Vec<String>, ServerFnError> {
    Ok(text_editor_service::autocomplete_path(kind, prefix, path)?)
}

#[nameth]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PathSelector {
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
