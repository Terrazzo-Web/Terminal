use server_fn::ServerFnError;
use terrazzo::server;

mod text_editor_service;
pub mod text_editor_ui;

#[server]
pub async fn base_path_autocomplete(path: String) -> Result<Vec<String>, ServerFnError> {
    Ok(text_editor_service::base_path_autocomplete(path)?)
}
