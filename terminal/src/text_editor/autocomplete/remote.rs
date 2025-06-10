#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_server::server::Server;

use crate::backend::client_service::remote_fn;
use crate::backend::client_service::remote_fn::RemoteFn;
use crate::backend::client_service::remote_fn::RemoteFnResult;
use crate::text_editor::path_selector::PathSelector;

pub static AUTOCOMPLETE_PATH_REMOTE_FN: RemoteFn = RemoteFn {
    name: super::AUTOCOMPLETE_PATH,
    callback: autocomplete_path,
};

inventory::submit! { AUTOCOMPLETE_PATH_REMOTE_FN }

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompletePathRequest {
    pub kind: PathSelector,
    pub prefix: Arc<str>,
    pub input: String,
}

fn autocomplete_path(server: &Server, arg: &str) -> RemoteFnResult {
    let autocomplete_path = remote_fn::uplift(|_server, arg: AutoCompletePathRequest| {
        ready(super::service::autocomplete_path(
            arg.kind,
            &arg.prefix,
            &arg.input,
        ))
    });
    Box::pin(autocomplete_path(server, arg))
}
