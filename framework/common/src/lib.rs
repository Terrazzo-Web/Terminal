#![cfg_attr(
    any(feature = "client", feature = "server"),
    deny(unused_crate_dependencies)
)]

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use self::client::*;

#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use self::server::*;
