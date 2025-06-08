use std::sync::Arc;

use server_fn::ServerFnError;
use terrazzo::server;

use super::path_selector::PathSelector;

mod service;
pub mod ui;

#[server]
async fn autocomplete_path(
    kind: PathSelector,
    prefix: Arc<str>,
    input: String,
) -> Result<Vec<String>, ServerFnError> {
    Ok(service::autocomplete_path(kind, &prefix, &input)?)
}
