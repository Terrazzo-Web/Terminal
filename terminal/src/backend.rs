#![cfg(feature = "server")]

use std::env::set_current_dir;
use std::sync::Arc;

use clap::Parser as _;
use cli::kill::KillServerError;
use daemonize::DaemonizeServerError;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use root_ca_config::PrivateRootCaError;
use server_config::TerminalBackendServer;
use tls_config::TlsConfigError;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use trz_gateway_common::handle::ServerStopError;
use trz_gateway_server::server::GatewayError;
use trz_gateway_server::server::Server;
use trz_gateway_server::server::gateway_config::GatewayConfig;
use trz_gateway_server::server::gateway_config::memoize::MemoizedGatewayConfig;

use self::cli::Action;
use self::cli::Cli;
use self::root_ca_config::PrivateRootCa;
use self::tls_config::make_tls_config;
use crate::assets;

mod cli;
mod daemonize;
mod root_ca_config;
mod server_config;
mod tls_config;

const HOST: &str = "127.0.0.1";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub fn run_server() -> Result<(), RunServerError> {
    let cli = {
        let mut cli = Cli::parse();
        cli.pidfile = cli.pidfile.replace("$port", &cli.port.to_string());
        cli
    };

    if cli.action == Action::Stop {
        return Ok(cli.kill()?);
    }

    let config = MemoizedGatewayConfig::new(true, || {
        let root_ca = PrivateRootCa::load(&cli)?;
        let tls_config = make_tls_config(&root_ca)?;
        let config = TerminalBackendServer {
            host: cli.host.clone(),
            port: cli.port,
            root_ca,
            tls_config,
        };

        if cli.action == Action::Start {
            self::daemonize::daemonize(cli)?;
        }
        Ok::<RunServerError>(Arc::new(config))
    });

    return run_server_async(config);
}

#[tokio::main]
async fn run_server_async(config: impl GatewayConfig) -> Result<(), RunServerError> {
    set_current_dir(std::env::var("HOME").expect("HOME")).map_err(RunServerError::SetCurrentDir)?;

    assets::install_assets();
    let (server, handle) = Server::run(config).await?;
    signal(SignalKind::terminate())
        .map_err(RunServerError::Signal)?
        .recv()
        .await;
    handle.stop("Quit").await?;
    drop(server);
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RunServerError {
    #[error("[{n}] {0}", n = self.name())]
    KillServer(#[from] KillServerError),

    #[error("[{n}] {0}", n = self.name())]
    PrivateRootCa(#[from] PrivateRootCaError),

    #[error("[{n}] {0}", n = self.name())]
    TlsConfig(#[from] TlsConfigError),

    #[error("[{n}] {0}", n = self.name())]
    Daemonize(#[from] DaemonizeServerError),

    #[error("[{n}] {0}", n = self.name())]
    Server(#[from] GatewayError<TerminalBackendServer>),

    #[error("[{n}] {0}", n = self.name())]
    SetCurrentDir(std::io::Error),

    #[error("[{n}] {0}", n = self.name())]
    Signal(std::io::Error),

    #[error("[{n}] {0}", n = self.name())]
    Stop(#[from] ServerStopError),
}
