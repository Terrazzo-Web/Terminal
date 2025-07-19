#![cfg(feature = "server")]

use std::future::ready;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;

use clap::Parser as _;
use futures::FutureExt as _;
use futures::future::Either;
use futures::future::Shared;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::autoclone;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::sync::oneshot;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_client::client::AuthCode;
use trz_gateway_client::client::Client;
use trz_gateway_client::client::NewClientError;
use trz_gateway_client::client::connect::ConnectError;
use trz_gateway_client::load_client_certificate::make_client_certificate;
use trz_gateway_client::load_client_certificate::store_client_certificate;
use trz_gateway_client::tunnel_config::TunnelConfig;
use trz_gateway_common::crypto_provider::crypto_provider;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::has_diff::DiffOption;
use trz_gateway_common::handle::ServerHandle;
use trz_gateway_common::handle::ServerStopError;
use trz_gateway_common::security_configuration::SecurityConfig;
use trz_gateway_common::security_configuration::certificate::CertificateConfig;
use trz_gateway_common::security_configuration::either::EitherConfig;
use trz_gateway_common::security_configuration::trusted_store::native::NativeTrustedStoreConfig;
use trz_gateway_common::x509::time::asn1_to_system_time;
use trz_gateway_server::server::GatewayError;
use trz_gateway_server::server::Server;
use trz_gateway_server::server::acme::active_challenges::ActiveChallenges;
use trz_gateway_server::server::acme::certificate_config::AcmeCertificateConfig;

use self::agent::AgentTunnelConfig;
use self::auth::AuthConfig;
use self::cli::Action;
use self::cli::Cli;
use self::config::Config;
use self::config::ConfigFile;
use self::config::DynConfig;
use self::config::io::ConfigFileError;
use self::config::kill::KillServerError;
use self::config::password::SetPasswordError;
use self::daemonize::DaemonizeServerError;
use self::root_ca_config::PrivateRootCa;
use self::root_ca_config::PrivateRootCaError;
use self::server_config::TerminalBackendServer;
use self::tls_config::TlsConfigError;
use self::tls_config::make_tls_config;
use crate::assets;
use crate::backend::client_service::remote_fn_service;
use crate::backend::config::mesh::DynamicMeshConfig;
use crate::utils::more_path::MorePath as _;

mod agent;
pub mod auth;
mod cli;
pub mod client_service;
pub mod config;
mod daemonize;
pub mod protos;
mod root_ca_config;
mod server_config;
pub mod throttling_stream;
mod tls_config;

const HOST: &str = "localhost";
const PORT: u16 = if cfg!(debug_assertions) { 3000 } else { 3001 };
const MIN_CERTIFICATE_RENEWAL_DELAY: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(5)
} else {
    Duration::from_secs(60 * 5)
};

pub fn run_server() -> Result<(), RunServerError> {
    crypto_provider();
    let cli = {
        let mut cli = Cli::parse();
        if let Some(config_file) = &mut cli.config_file {
            if Path::new(config_file).is_relative() {
                let concat: PathBuf = [&home(), ".terrazzo", config_file].iter().collect();
                *config_file = concat.to_owned_string()
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
    println!("Config: {:#?}", config);

    if cli.action == Action::Stop {
        return Ok(config.server.kill()?);
    }

    if cli.action == Action::Restart {
        config.server.kill()?;
    }

    std::env::set_current_dir(home()).map_err(RunServerError::SetCurrentDir)?;
    if cli.action == Action::Start {
        self::daemonize::daemonize(&config.server)?;
    }

    return run_server_async(cli, config);
}

#[tokio::main]
async fn run_server_async(cli: Cli, config: Config) -> Result<(), RunServerError> {
    #[cfg(debug_assertions)]
    {
        println!("server_fn paths:");
        for (m, p) in server_fn::axum::server_fn_paths() {
            println!("\t{m} {p}");
        }
        println!("server_fn paths END");
    }
    let config = config.into_dyn(&cli);
    let server_config = config.server.clone();
    if cli.action == Action::SetPassword {
        return Ok(server_config.set_password()?);
    }

    let backend_config = {
        let root_ca = PrivateRootCa::load(&config)?;
        let active_challenges = ActiveChallenges::default();

        let tls_config = {
            let root_ca = root_ca.clone();
            let active_challenges = active_challenges.clone();
            let dynamic_acme_config = config.letsencrypt.clone();
            let joined_config = config.letsencrypt.zip(
                &config
                    .server
                    .view_diff(|server_config| server_config.certificate_renewal_threshold),
            );
            joined_config.view(move |(letsencrypt, certificate_renewal_threshold)| {
                debug!("Refresh TLS config");
                if let Some(letsencrypt) = &**letsencrypt {
                    EitherConfig::Right(SecurityConfig {
                        trusted_store: NativeTrustedStoreConfig,
                        certificate: AcmeCertificateConfig::new(
                            dynamic_acme_config.clone(),
                            letsencrypt.clone(),
                            active_challenges.clone(),
                            *certificate_renewal_threshold,
                        ),
                    })
                } else {
                    EitherConfig::Left(make_tls_config(&root_ca).unwrap())
                }
            })
        };

        TerminalBackendServer {
            config,
            root_ca,
            tls_config,
            auth_config: server_config
                .view(|server| DiffArc::from(AuthConfig::new(server)))
                .into(),
            active_challenges,
        }
    };

    assets::install::install_assets();
    let config = backend_config.config.clone();
    let (server, server_handle, crash) = Server::run(backend_config).await?;
    remote_fn_service::setup(&server);
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
        biased;
        crash = crash.clone() => {
            server_handle.stop(crash).await?;
        }
        _ = terminate.recv() => {
            server_handle.stop("Quit").await?;
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

#[autoclone]
async fn run_client_async(
    cli: Cli,
    config: DiffArc<DynConfig>,
    server: Arc<Server>,
) -> Result<ServerHandle<()>, RunClientError> {
    let (shutdown_rx, terminated_tx, handle) = ServerHandle::new("Dynamic Client");
    let auth_code = AuthCode::from(cli.auth_code);
    let (terminated_all_tx, terminated_all_rx) = oneshot::channel::<()>();
    let terminated_all_tx = Arc::new(terminated_all_tx);

    let dynamic_mesh_config = config.mesh.clone();
    let dynamic_client = config.mesh.view(move |mesh| {
        debug!("Refresh mesh config");
        if let Some(mesh) = (**mesh).clone() {
            let auth_code = Arc::new(Mutex::new(auth_code.clone()));

            let (abort_client_tx, abort_client_rx) = oneshot::channel();
            let abort_client_rx = abort_client_rx.shared();
            let client_task = async move {
                autoclone!(server, terminated_all_tx, dynamic_mesh_config);
                let Some(agent_config) = AgentTunnelConfig::new(auth_code, &mesh, &server).await
                else {
                    info!("Gateway client disabled");
                    return Err(RunClientError::ClientNotEnabled);
                };
                let agent_config = Arc::new(agent_config);
                info!(?agent_config, "Gateway client enabled");

                let schedule_client_certificate_renewal_task = schedule_client_certificate_renewal(
                    abort_client_rx.clone(),
                    dynamic_mesh_config.clone(),
                    mesh.client_certificate_renewal,
                    agent_config.clone(),
                );
                tokio::spawn(
                    schedule_client_certificate_renewal_task.instrument(info_span!(
                        "Certificate renewal",
                        id = next_client_certificate_renewal_schedule_id()
                    )),
                );

                let client = Client::new(agent_config)?;
                let client_handle = client.run().await?;
                let client_handle_task = async move {
                    let _ = abort_client_rx.await;
                    match client_handle.stop("Updated mesh config").await {
                        Ok(()) => debug!("The client was successfully stopped"),
                        Err(error) => warn!("Failed to stop client: {error}"),
                    };
                };
                tokio::spawn(client_handle_task.instrument(info_span!("Handle")));
                drop(terminated_all_tx);

                return Ok(());
            };
            tokio::spawn(client_task.instrument(info_span!("Client")));
            DiffOption::from(DiffArc::from(abort_client_tx))
        } else {
            None.into()
        }
    });

    tokio::spawn(async move {
        let () = shutdown_rx.await;
        drop(dynamic_client);
        let _terminated = terminated_all_rx.await;
        let _ = terminated_tx.send(());
    });

    return Ok(handle);
}

async fn schedule_client_certificate_renewal(
    abort_client_rx: Shared<oneshot::Receiver<()>>,
    dynamic_mesh_config: DynamicMeshConfig,
    client_certificate_renewal: Duration,
    client_config: impl TunnelConfig,
) {
    debug!("Start");
    defer!(debug!("Canceled"));
    loop {
        let client_certificate = client_config.client_certificate();
        let expiration = if let Ok(Ok(expiration)) = client_certificate
            .certificate()
            .map(|certificate| asn1_to_system_time(certificate.certificate.not_after()))
        {
            expiration
        } else {
            debug!("Failed to parse certificat not_after date");
            SystemTime::UNIX_EPOCH
        };
        let now = SystemTime::now();
        let renew_in = if expiration <= now + client_certificate_renewal {
            info!(
                "Certificate is already expiring expiration:{expiration} <= now:{now} + client_certificate_renewal:{client_certificate_renewal}",
                expiration = humantime::format_rfc3339(expiration),
                now = humantime::format_rfc3339(now),
                client_certificate_renewal = humantime::format_duration(client_certificate_renewal),
            );
            MIN_CERTIFICATE_RENEWAL_DELAY
        } else {
            let renew_at = expiration - client_certificate_renewal;
            renew_at
                .duration_since(now)
                .unwrap_or(MIN_CERTIFICATE_RENEWAL_DELAY)
        };

        debug!(
            "Renewing client certificate in {}",
            humantime::format_duration(renew_in)
        );
        match futures::future::select(
            abort_client_rx.clone(),
            Box::pin(tokio::time::sleep(renew_in)),
        )
        .await
        {
            Either::Left((_abort, _sleep)) => return,
            Either::Right(((), _abort_client_rx)) => {}
        }

        info!("Renewing client certificate");
        let auth_code = client_config.current_auth_code().lock().unwrap().clone();
        let Ok(new_certificate) = make_client_certificate(&client_config, auth_code)
            .await
            .inspect_err(|error| warn!("Failed to renew client certificate: {error}"))
        else {
            continue;
        };
        let renewed = dynamic_mesh_config.with(|mesh| {
            if let Some(mesh) = &**mesh {
                let result = store_client_certificate(
                    mesh.client_certificate_paths().as_ref(),
                    new_certificate,
                );
                return result
                    .inspect(|_pem| info!("Renewed the client certificate"))
                    .inspect_err(|error| {
                        warn!("Failed to store the new client certificate: {error}")
                    })
                    .is_ok();
            }
            false
        });
        if renewed {
            // Force restart the client
            let dynamic_mesh_config = dynamic_mesh_config.clone();
            tokio::spawn(async move {
                dynamic_mesh_config.set(|mesh| mesh.clone());
            });
        }
    }
}

fn next_client_certificate_renewal_schedule_id() -> i32 {
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering::SeqCst;
    static NEXT: AtomicI32 = AtomicI32::new(1);
    NEXT.fetch_add(1, SeqCst)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RunClientError {
    #[error("[{n}] Not running Gateway Client", n = self.name())]
    ClientNotEnabled,

    #[error("[{n}] {0}", n = self.name())]
    NewClient(#[from] NewClientError<Arc<AgentTunnelConfig>>),

    #[error("[{n}] {0}", n = self.name())]
    RunClientError(#[from] ConnectError),

    #[error("[{n}] {0}", n = self.name())]
    Aborted(String),
}

fn home() -> String {
    std::env::var("HOME").expect("HOME")
}
