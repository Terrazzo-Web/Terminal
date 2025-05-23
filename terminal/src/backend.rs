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
use trz_gateway_common::security_configuration::certificate::dynamic::DynamicCertificate;
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
use self::config_file::Config;
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

    let config = if let Some(path) = cli.config_file.as_deref() {
        ConfigFile::load(path)?
    } else {
        ConfigFile::default()
    }
    .merge(&cli);

    #[cfg(debug_assertions)]
    println!("Config: {:#?}", config.get());

    if cli.action == Action::Stop {
        return Ok(config.server.get().kill()?);
    }

    if cli.action == Action::SetPassword {
        return Ok(config.server.set_password()?);
    }

    let root_ca = PrivateRootCa::load(&config)?;
    let active_challenges = ActiveChallenges::default();

    let tls_config = {
        let root_ca = root_ca.clone();
        let active_challenges = active_challenges.clone();
        let dynamic_acme_config = config.letsencrypt.clone();
        config.letsencrypt.view(move |letsencrypt| {
            if let Some(letsencrypt) = &letsencrypt {
                EitherConfig::Right(SecurityConfig {
                    trusted_store: NativeTrustedStoreConfig,
                    certificate: AcmeCertificateConfig::new(
                        dynamic_acme_config.clone(),
                        letsencrypt.clone(),
                        active_challenges.clone(),
                    ),
                })
            } else {
                EitherConfig::Left(make_tls_config(&root_ca).unwrap())
            }
        })
    };
    let tls_config = Arc::new(DynamicCertificate::from(tls_config));
    let backend_config = TerminalBackendServer {
        root_ca,
        tls_config,
        auth_config: config.view(|config| Arc::new(AuthConfig::new(&config.server))),
        active_challenges,
        config,
    };

    if cli.action == Action::Start {
        let server_config = &backend_config.config.server;
        server_config.with(|server_config| self::daemonize::daemonize(server_config))?;
    }

    if let Some(path) = cli.config_file.as_deref() {
        let () = backend_config.config.to_config_file().save(path)?;
    }
    return run_server_async(cli, backend_config);
}

#[tokio::main]
async fn run_server_async(
    cli: Cli,
    backend_config: TerminalBackendServer,
) -> Result<(), RunServerError> {
    std::env::set_current_dir(home()).map_err(RunServerError::SetCurrentDir)?;

    assets::install_assets();
    let config = backend_config.config.clone();
    let (server, server_handle, crash) = Server::run(backend_config).await?;
    let crash = crash
        .then(|crash| {
            let crash = crash
                .map(|crash| format!("Crashed: {crash}"))
                .unwrap_or_else(|_| "Server task dropped".to_owned());
            ready(crash)
        })
        .shared();

    let client_handle = async {
        match run_client_async(cli, config, server.clone()).await {
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
    config: Config,
    server: Arc<Server>,
) -> Result<ServerHandle<()>, RunClientError> {
    let _span = info_span!("Client").entered();
    let Some(agent_config) = AgentTunnelConfig::new(&cli, config, server).await else {
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
