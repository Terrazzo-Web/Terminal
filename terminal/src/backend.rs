#![cfg(feature = "server")]

use std::env::set_current_dir;
use std::iter::once;

use clap::Parser as _;
use terrazzo::axum;
use terrazzo::axum::Router;
use terrazzo::axum::extract::Path;
use terrazzo::axum::routing::get;
use terrazzo::http::header::AUTHORIZATION;
use terrazzo::static_assets;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing::enabled;

use self::cli::Action;
use self::cli::Cli;
use crate::api;
use crate::assets;

mod cli;
mod daemonize;

const HOST: &str = "127.0.0.1";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub fn run_server() -> std::io::Result<()> {
    let cli = {
        let mut cli = Cli::parse();
        cli.pidfile = cli.pidfile.replace("$port", &cli.port.to_string());
        cli
    };

    if cli.action == Action::Stop {
        return cli.kill();
    }

    let address = format!("{}:{}", cli.host, cli.port);
    println!("Listening on http://{address}");

    if cli.action == Action::Start {
        self::daemonize::daemonize(cli)?;
    }

    run_server_async(&address)
}

#[tokio::main]
async fn run_server_async(address: &str) -> std::io::Result<()> {
    set_current_dir(std::env::var("HOME").expect("HOME"))?;

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

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await
}
