use std::sync::Arc;

use server_fn::ServerFnError;
use terrazzo::server;

mod service;

#[server]
pub async fn load_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<Arc<str>>, ServerFnError> {
    service::load_file(base_path, file_path)
}

#[server]
pub async fn store_file(
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
) -> Result<(), ServerFnError> {
    service::store_file(base_path, file_path, content)
}
