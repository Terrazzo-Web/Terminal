#![cfg(feature = "server")]
#![deny(unused_crate_dependencies)]

use std::env::set_current_dir;
use std::iter::once;
use std::path::PathBuf;

use axum::Router;
use http::header::AUTHORIZATION;
use terrazzo as _;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;
use tower_http::trace::TraceLayer;
use tracing::enabled;
use tracing::Level;

const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

mod api;
mod processes;
mod terminal_id;

#[tokio::main]
async fn main() {
    set_current_dir(std::env::var("HOME").expect("HOME")).expect("set_current_dir");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .init();

    let target_asset_dir = {
        let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let debug_or_release = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        cargo_manifest_dir
            .join("target")
            .join(debug_or_release)
            .join("assets")
    };

    let router = Router::new()
        .nest_service("/", ServeFile::new(target_asset_dir.join("index.html")))
        .nest_service("/assets", ServeDir::new(target_asset_dir))
        .nest_service("/api", api::server::route());
    let router = router.layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)));
    let router = if enabled!(Level::TRACE) {
        router.layer(TraceLayer::new_for_http())
    } else {
        router
    };
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{PORT}"))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}
