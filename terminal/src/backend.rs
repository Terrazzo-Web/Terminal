#![cfg(feature = "server")]

use std::env::set_current_dir;
use std::fs::File;
use std::iter::once;
use std::num::NonZeroI32;

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

const HOST: &str = "127.0.0.1";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub fn run_server() -> std::io::Result<()> {
    let address = format!("{HOST}:{PORT}");
    println!("Listening on http://{address}");

    match fork()? {
        Some(_pid) => std::process::exit(0),
        None => { /* in the child process */ }
    }
    check_err(unsafe { libc::setsid() }, |r| r != 1)?;

    match fork()? {
        Some(pid) => {
            println!("Child pid is {pid}");
            std::process::exit(0);
        }
        None => { /* in the child process */ }
    }

    File::open(path)

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

fn fork() -> std::io::Result<Option<NonZeroI32>> {
    let pid = check_err(unsafe { libc::fork() }, |pid| pid != -1)?;
    return Ok(NonZeroI32::new(pid));
}

fn check_err<IsOk: FnOnce(R) -> bool, R: Copy>(result: R, is_ok: IsOk) -> std::io::Result<R> {
    if is_ok(result) {
        Ok(result)
    } else {
        Err(std::io::Error::last_os_error())
    }
}
