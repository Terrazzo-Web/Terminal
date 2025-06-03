mod api;
mod assets;
mod backend;
mod frontend;
mod processes;
mod terminal;
mod terminal_id;
mod text_editor;

#[cfg(feature = "server")]
pub use self::backend::RunServerError;
#[cfg(feature = "server")]
pub use self::backend::run_server;

#[allow(unused)]
// #[server]
async fn dummy() -> Result<(), server_fn::ServerFnError> {
    Ok(())
}
