#![cfg(feature = "server")]

use std::env::set_current_dir;

use agent::AgentTunnelConfig;
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
use tracing::info;
use tracing::info_span;
use trz_gateway_client::client::Client;
use trz_gateway_client::client::NewClientError;
use trz_gateway_client::client::connect::ConnectError;
use trz_gateway_common::crypto_provider::crypto_provider;
use trz_gateway_common::handle::ServerHandle;
use trz_gateway_common::handle::ServerStopError;
use trz_gateway_server::server::GatewayError;
use trz_gateway_server::server::Server;

use self::cli::Action;
use self::cli::Cli;
use self::root_ca_config::PrivateRootCa;
use self::tls_config::make_tls_config;
use crate::assets;

mod agent;
mod cli;
mod daemonize;
mod root_ca_config;
mod server_config;
mod tls_config;

const HOST: &str = "localhost";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub fn run_server() -> Result<(), RunServerError> {
    crypto_provider();
    let cli = {
        let mut cli = Cli::parse();
        cli.pidfile = cli.pidfile.replace("$port", &cli.port.to_string());
        cli
    };

    if cli.action == Action::Stop {
        return Ok(cli.kill()?);
    }

    let root_ca = PrivateRootCa::load(&cli)?;
    let tls_config = make_tls_config(&root_ca)?;
    let config = TerminalBackendServer {
        host: cli.host.clone(),
        port: cli.port,
        root_ca,
        tls_config,
    };

    if cli.action == Action::Start {
        self::daemonize::daemonize(&cli)?;
    }

    return run_server_async(cli, config);
}

#[tokio::main]
async fn run_server_async(cli: Cli, config: TerminalBackendServer) -> Result<(), RunServerError> {
    set_current_dir(std::env::var("HOME").expect("HOME")).map_err(RunServerError::SetCurrentDir)?;

    assets::install_assets();
    let (server, server_handle) = Server::run(config).await?;

    let client_handle = match run_client_async(cli).await {
        Ok(client_handle) => Some(client_handle),
        Err(RunClientError::ClientNotEnabled) => None,
        Err(error) => return Err(error)?,
    };

    signal(SignalKind::terminate())
        .map_err(RunServerError::Signal)?
        .recv()
        .await;
    server_handle.stop("Quit").await?;
    drop(server);

    if let Some(client_handle) = client_handle {
        client_handle.stop("Quit").await?;
    }

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

    #[error("[{n}] {0}", n = self.name())]
    RunClient(#[from] RunClientError),
}

async fn run_client_async(cli: Cli) -> Result<ServerHandle<()>, RunClientError> {
    let _span = info_span!("Client").entered();
    let Some(agent_config) = AgentTunnelConfig::new(&cli).await else {
        info!("Gateway client disabled");
        return Err(RunClientError::ClientNotEnabled);
    };
    info!(?agent_config, "Gateway client enabled");
    let client = Client::new(agent_config)?;
    Ok(client.run().await?)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RunClientError {
    #[error("[{n}] Not running Gateway Client", n = self.name())]
    ClientNotEnabled,

    #[error("[{n}] {0}", n = self.name())]
    NewClient(#[from] NewClientError<AgentTunnelConfig>),

    #[error("[{n}] {0}", n = self.name())]
    RunClientError(#[from] ConnectError),
}
