#![cfg(feature = "server")]

use std::future::ready;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser as _;
use futures::FutureExt as _;
use nameth::NamedEnumValues as _;
use nameth::nameth;
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
use trz_gateway_common::security_configuration::SecurityConfig;
use trz_gateway_common::security_configuration::either::EitherConfig;
use trz_gateway_common::security_configuration::trusted_store::native::NativeTrustedStoreConfig;
use trz_gateway_server::server::GatewayError;
use trz_gateway_server::server::Server;
use trz_gateway_server::server::acme::active_challenges::ActiveChallenges;
use trz_gateway_server::server::acme::certificate_config::AcmeCertificateConfig;

use self::agent::AgentTunnelConfig;
use self::auth::AuthConfig;
use self::cli::Action;
use self::cli::Cli;
use self::config_file::ConfigFile;
use self::config_file::io::ConfigFileError;
use self::config_file::kill::KillServerError;
use self::config_file::password::SetPasswordError;
use self::daemonize::DaemonizeServerError;
use self::root_ca_config::PrivateRootCa;
use self::root_ca_config::PrivateRootCaError;
use self::server_config::TerminalBackendServer;
use self::tls_config::TlsConfigError;
use self::tls_config::make_tls_config;
use crate::assets;

mod agent;
pub mod auth;
mod cli;
pub mod client_service;
pub mod config_file;
mod daemonize;
pub mod protos;
mod root_ca_config;
mod server_config;
pub mod throttling_stream;
mod tls_config;

const HOST: &str = "localhost";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };

pub fn run_server() -> Result<(), RunServerError> {
    crypto_provider();
    let cli = {
        let mut cli = Cli::parse();
        if let Some(config_file) = &mut cli.config_file {
            if Path::new(config_file).is_relative() {
                let concat: PathBuf = [&home(), ".terrazzo", config_file].iter().collect();
                *config_file = concat.to_string_lossy().to_string()
            }
        }
        cli
    };

    let config_file = if let Some(path) = cli.config_file.as_deref() {
        ConfigFile::load(path)?
    } else {
        ConfigFile::default()
    }
    .merge(&cli);

    #[cfg(debug_assertions)]
    println!("Config: {config_file:#?}");

    if cli.action == Action::Stop {
        return Ok(config_file.server.kill()?);
    }

    if cli.action == Action::SetPassword {
        return Ok(config_file.set_password(cli.config_file)?);
    }

    let root_ca = PrivateRootCa::load(&config_file)?;

    let active_challenges = ActiveChallenges::default();
    let tls_config = if let Some(acme_config) = &config_file.letsencrypt {
        // TODO: provide a callback to set the account creds.
        EitherConfig::Right(SecurityConfig {
            trusted_store: NativeTrustedStoreConfig,
            certificate: AcmeCertificateConfig::new(acme_config.clone(), active_challenges.clone()),
        })
    } else {
        EitherConfig::Left(make_tls_config(&root_ca)?)
    };
    let config_file = Arc::new(config_file);
    let client_name = if let Some(mesh) = &config_file.mesh {
        Some(mesh.client_name.as_str().into())
    } else {
        None
    };
    let config = TerminalBackendServer {
        client_name,
        host: config_file.server.host.clone(),
        port: config_file.server.port,
        root_ca,
        tls_config,
        auth_config: AuthConfig::new(&config_file).into(),
        config_file: config_file.clone(),
        active_challenges,
    };

    if cli.action == Action::Start {
        self::daemonize::daemonize(&config.config_file)?;
    }

    if let Some(path) = cli.config_file.as_deref() {
        let () = config_file.to_config_file().save(path)?;
    }
    return run_server_async(cli, config_file, config);
}

#[tokio::main]
async fn run_server_async(
    cli: Cli,
    config_file: Arc<ConfigFile>,
    config: TerminalBackendServer,
) -> Result<(), RunServerError> {
    std::env::set_current_dir(home()).map_err(RunServerError::SetCurrentDir)?;

    assets::install_assets();
    let (server, server_handle, crash) = Server::run(config).await?;
    let crash = crash
        .then(|crash| {
            let crash = crash
                .map(|crash| format!("Crashed: {crash}"))
                .unwrap_or_else(|_| "Server task dropped".to_owned());
            ready(crash)
        })
        .shared();

    let client_handle = async {
        match run_client_async(cli, config_file, server.clone()).await {
            Ok(client_handle) => Ok(Some(client_handle)),
            Err(RunClientError::ClientNotEnabled) => Ok(None),
            Err(error) => Err(error),
        }
    };
    let client_handle = tokio::select! {
        h = client_handle => h,
        crash = crash.clone() => Err(RunClientError::Aborted(crash)),
    }?;

    let mut terminate = signal(SignalKind::terminate()).map_err(RunServerError::Signal)?;
    tokio::select! {
        _ = terminate.recv() => {
            server_handle.stop("Quit").await?;
        }
        crash = crash.clone() => {
            server_handle.stop(crash).await?;
        }
    }
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
    ConfigFile(#[from] ConfigFileError),

    #[error("[{n}] {0}", n = self.name())]
    SetPassword(#[from] SetPasswordError),

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

async fn run_client_async(
    cli: Cli,
    config_file: Arc<ConfigFile>,
    server: Arc<Server>,
) -> Result<ServerHandle<()>, RunClientError> {
    let _span = info_span!("Client").entered();
    let Some(agent_config) = AgentTunnelConfig::new(&cli, &config_file, server).await else {
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

    #[error("[{n}] {0}", n = self.name())]
    Aborted(String),
}

fn home() -> String {
    std::env::var("HOME").expect("HOME")
}
