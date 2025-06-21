use std::ops::Deref;
use std::path::PathBuf;
use std::time::Duration;

use tracing::warn;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::has_diff::DiffOption;
use trz_gateway_common::retry_strategy::RetryStrategy;

use super::Config;
use super::ConfigFile;
use super::ConfigImpl;
use super::mesh::MeshConfig;
use super::server::ServerConfig;
use super::types::ConfigFileTypes;
use super::types::RuntimeTypes;
use crate::backend::HOST;
use crate::backend::PORT;
use crate::backend::auth::DEFAULT_TOKEN_LIFETIME;
use crate::backend::auth::DEFAULT_TOKEN_REFRESH;
use crate::backend::cli::Cli;
use crate::backend::home;

impl ConfigFile {
    pub fn merge(self, cli: &Cli) -> Config {
        Config(ConfigImpl {
            server: merge_server_config(&self.server, cli).into(),
            mesh: merge_mesh_config(self.mesh.as_deref(), cli).into(),
            letsencrypt: self.letsencrypt.clone(),
        })
    }
}

impl Config {
    pub fn to_config_file(&self) -> ConfigFile {
        let ConfigImpl {
            server,
            mesh,
            letsencrypt,
        } = self.deref();
        ConfigFile(ConfigImpl {
            server: DiffArc::from(ServerConfig {
                host: Some(server.host.clone()),
                port: Some(server.port),
                pidfile: Some(server.pidfile.clone()),
                private_root_ca: Some(server.private_root_ca.clone()),
                password: server.password.clone(),
                token_lifetime: Some(humantime::format_duration(server.token_lifetime).to_string()),
                token_refresh: Some(humantime::format_duration(server.token_refresh).to_string()),
                config_file_poll_strategy: Some(server.config_file_poll_strategy.clone()),
                certificate_renewal_threshold: Some(
                    humantime::format_duration(server.certificate_renewal_threshold).to_string(),
                ),
            }),
            mesh: DiffOption::from(mesh.as_ref().map(|mesh| {
                DiffArc::from(MeshConfig {
                    client_name: Some(mesh.client_name.clone()),
                    gateway_url: Some(mesh.gateway_url.clone()),
                    gateway_pki: mesh.gateway_pki.clone(),
                    client_certificate: Some(mesh.client_certificate.clone()),
                    retry_strategy: Some(mesh.retry_strategy.clone()),
                })
            })),
            letsencrypt: letsencrypt.clone(),
        })
    }
}

fn merge_server_config(
    server: &ServerConfig<ConfigFileTypes>,
    cli: &Cli,
) -> ServerConfig<RuntimeTypes> {
    let port = cli.port.or(server.port).unwrap_or(PORT);
    ServerConfig {
        host: {
            let host = cli.host.as_deref();
            let host = host.or(server.host.as_deref());
            host.unwrap_or(HOST).to_owned()
        },
        port,
        pidfile: {
            let pidfile = cli.pidfile.as_deref();
            let pidfile = pidfile.or(server.pidfile.as_deref()).map(str::to_owned);
            pidfile.unwrap_or_else(|| {
                [home(), format!(".terrazzo/terminal-{port}.pid")]
                    .iter()
                    .collect::<PathBuf>()
                    .to_string_lossy()
                    .to_string()
            })
        },
        private_root_ca: {
            let private_root_ca = cli.private_root_ca.as_deref();
            let private_root_ca = private_root_ca
                .or(server.private_root_ca.as_deref())
                .map(str::to_owned);
            private_root_ca.unwrap_or_else(|| {
                [&home(), ".terrazzo/root_ca"]
                    .iter()
                    .collect::<PathBuf>()
                    .to_string_lossy()
                    .to_string()
            })
        },
        password: server.password.clone(),
        token_lifetime: parse_duration(server.token_lifetime.as_deref())
            .unwrap_or(DEFAULT_TOKEN_LIFETIME),
        token_refresh: parse_duration(server.token_refresh.as_deref())
            .unwrap_or(DEFAULT_TOKEN_REFRESH),
        config_file_poll_strategy: server
            .config_file_poll_strategy
            .clone()
            .unwrap_or_else(|| RetryStrategy::fixed(Duration::from_secs(60))),
        certificate_renewal_threshold: parse_duration(
            server.certificate_renewal_threshold.as_deref(),
        )
        .unwrap_or(Duration::from_secs(1) * 3600 * 24 * 30),
    }
}

fn parse_duration(duration: Option<&str>) -> Option<Duration> {
    duration.and_then(|duration| {
        humantime::parse_duration(duration)
            .inspect_err(|error| warn!("Failed to parse '{duration}': {error}"))
            .ok()
    })
}

fn merge_mesh_config(
    mesh: Option<&MeshConfig<ConfigFileTypes>>,
    cli: &Cli,
) -> Option<DiffArc<MeshConfig<RuntimeTypes>>> {
    let mesh = mesh.as_ref();
    let client_name = cli.client_name.as_ref().cloned();
    let gateway_url = cli.gateway_url.as_ref().cloned();
    let gateway_pki = cli.gateway_pki.as_ref().cloned();
    let client_certificate = cli.client_certificate.as_ref().cloned();
    Some(DiffArc::from(MeshConfig {
        client_name: client_name.or(mesh.and_then(|m| m.client_name.to_owned()))?,
        gateway_url: gateway_url.or(mesh.and_then(|m| m.gateway_url.to_owned()))?,
        gateway_pki: gateway_pki.or(mesh.and_then(|m| m.gateway_pki.to_owned())),
        client_certificate: client_certificate
            .or(mesh.and_then(|m| m.client_certificate.to_owned()))
            .unwrap_or_else(|| {
                [&home(), ".terrazzo/client_certificate"]
                    .iter()
                    .collect::<PathBuf>()
                    .to_string_lossy()
                    .to_string()
            }),
        retry_strategy: mesh
            .and_then(|mesh| mesh.retry_strategy.clone())
            .unwrap_or_default(),
    }))
}
