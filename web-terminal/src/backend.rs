#![cfg(feature = "server")]

use std::env::set_current_dir;
use std::iter::once;

use terrazzo::axum;
use terrazzo::axum::extract::Path;
use terrazzo::axum::routing::get;
use terrazzo::axum::Router;
use terrazzo::http::header::AUTHORIZATION;
use terrazzo::static_assets;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tracing::enabled;
use tracing::Level;

use crate::api;
use crate::assets;

const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub async fn run_server() {
    set_current_dir(std::env::var("HOME").expect("HOME")).expect("set_current_dir");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(true)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .init();

    assets::install_assets();
    let router = Router::new()
        .route("/", get(|| static_assets::get("index.html")))
        .route(
            "/static/*file",
            get(|Path(path): Path<String>| static_assets::get(&path)),
        )
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
