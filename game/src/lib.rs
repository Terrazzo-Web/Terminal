mod assets;
mod backend;
mod frontend;
mod game;

#[cfg(feature = "server")]
pub use self::backend::run_server;
