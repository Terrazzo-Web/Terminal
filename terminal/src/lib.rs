#![cfg_attr(
    all(feature = "client", not(feature = "diagnostics")),
    allow(unused, clippy::unnecessary_lazy_evaluations, clippy::single_match)
)]

mod api;
mod assets;
mod backend;
mod converter;
mod frontend;
mod processes;
mod state;
mod terminal;
mod terminal_id;
mod text_editor;
mod utils;

#[cfg(test)]
use fluent_asserter as _;

#[cfg(feature = "server")]
pub use self::backend::RunServerError;
#[cfg(feature = "server")]
pub use self::backend::run_server;
