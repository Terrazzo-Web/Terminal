use std::future::ready;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

use crate::api::client_address::ClientAddress;
use crate::backend::client_service::remote_server_fn;
use crate::backend::client_service::remote_server_fn::RemoteServerFnResult;
use crate::text_editor::path_selector::PathSelector;

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompletePathArg {
    pub address: ClientAddress,
    pub kind: PathSelector,
    pub prefix: Arc<str>,
    pub input: String,
}

pub fn autocomplete_path(arg: String) -> RemoteServerFnResult {
    Box::pin(remote_server_fn::call(
        |_server, arg: AutoCompletePathArg| {
            ready(super::service::autocomplete_path(
                arg.kind,
                &arg.prefix,
                &arg.input,
            ))
        },
        arg,
    ))
}
