#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

use super::server_fn::AutocompleteItem;
use crate::backend::client_service::remote_fn_service;
use crate::text_editor::path_selector::schema::PathSelector;

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompletePathRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "k"))]
    pub kind: PathSelector,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "p"))]
    pub prefix: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "i"))]
    pub input: String,
}

remote_fn_service::declare_remote_fn!(
    AUTOCOMPLETE_PATH_REMOTE_FN,
    super::server_fn::AUTOCOMPLETE_PATH,
    AutoCompletePathRequest,
    Vec<AutocompleteItem>,
    |_server, arg| {
        ready(super::service::autocomplete_path(
            arg.kind,
            &arg.prefix,
            &arg.input,
        ))
    }
);
