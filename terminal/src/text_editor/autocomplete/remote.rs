#![cfg(feature = "server")]

use std::future::ready;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

use crate::backend::client_service::remote_server_fn;
use crate::backend::client_service::remote_server_fn::RemoteServerFn;
use crate::backend::client_service::remote_server_fn::RemoteServerFnResult;
use crate::text_editor::path_selector::PathSelector;

pub static AUTOCOMPLETE_PATH_SERVER_FN: RemoteServerFn = RemoteServerFn {
    name: super::AUTOCOMPLETE_PATH,
    callback: autocomplete_path,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompletePathRequest {
    pub kind: PathSelector,
    pub prefix: Arc<str>,
    pub input: String,
}

fn autocomplete_path(arg: String) -> RemoteServerFnResult {
    Box::pin(remote_server_fn::call(
        |_server, arg: AutoCompletePathRequest| {
            ready(super::service::autocomplete_path(
                arg.kind,
                &arg.prefix,
                &arg.input,
            ))
        },
        arg,
    ))
}
