#![allow(unused_crate_dependencies)]

use tracing::Level;

fn main() {
    terrazzo_terminal::run_server().unwrap_or_else(|error| {
        if tracing::enabled!(Level::ERROR) {
            tracing::error!("{error}")
        } else {
            eprintln!("{error}")
        }
    })
}
