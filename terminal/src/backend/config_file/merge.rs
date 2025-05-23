use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_server::server::acme::DynamicAcmeConfig;

use super::Config;
use super::ConfigFile;
use super::ConfigImpl;
use super::mesh::DynamicMeshConfig;
use super::mesh::MeshConfig;
use super::server::DynamicServerConfig;
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
        let config = Arc::new(DynamicConfig::from(Arc::new(ConfigImpl {
            server: merge_server_config(&self.server, cli),
            mesh: merge_mesh_config(self.mesh.as_deref(), cli),
            letsencrypt: self.letsencrypt.clone(),
        })));
        Config {
            server: DynamicServerConfig::from(config.derive(
                |config| config.server.clone(),
                |config, server_config| {
                    if Arc::ptr_eq(&config.server, server_config) {
                        return None;
                    }
                    Some(Arc::new(ConfigImpl {
                        server: server_config.clone(),
                        mesh: config.mesh.clone(),
                        letsencrypt: config.letsencrypt.clone(),
                    }))
                },
            )),
            mesh: DynamicMeshConfig::from(config.derive(
                |config| config.mesh.clone(),
                |config, mesh_config| {
                    if option_ptr_eq(&config.mesh, mesh_config) {
                        return None;
                    }
                    Some(Arc::new(ConfigImpl {
                        server: config.server.clone(),
                        mesh: mesh_config.clone(),
                        letsencrypt: config.letsencrypt.clone(),
                    }))
                },
            )),
            letsencrypt: DynamicAcmeConfig::from(config.derive(
                |config| config.letsencrypt.clone(),
                |config, letsencrypt| {
                    if option_ptr_eq(&config.letsencrypt, letsencrypt) {
                        return None;
                    }
                    Some(Arc::new(ConfigImpl {
                        server: config.server.clone(),
                        mesh: config.mesh.clone(),
                        letsencrypt: letsencrypt.clone(),
                    }))
                },
            )),
            config,
        }
    }
}

fn option_ptr_eq<T>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(_), None) => false,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
    }
}

impl Config {
    pub fn to_config_file(&self) -> ConfigFile {
        let config = self.config.get();
        let ConfigImpl {
            server,
            mesh,
            letsencrypt,
        } = &*config;
        ConfigFile(ConfigImpl {
            server: Arc::new(ServerConfig {
                host: Some(server.host.clone()),
                port: Some(server.port),
                pidfile: Some(server.pidfile.clone()),
                private_root_ca: Some(server.private_root_ca.clone()),
                password: server.password.clone(),
                token_lifetime: Some(humantime::format_duration(server.token_lifetime).to_string()),
                token_refresh: Some(humantime::format_duration(server.token_refresh).to_string()),
            }),
            mesh: mesh.as_ref().map(|mesh| {
                Arc::new(MeshConfig {
                    client_name: Some(mesh.client_name.clone()),
                    gateway_url: Some(mesh.gateway_url.clone()),
                    gateway_pki: mesh.gateway_pki.clone(),
                    client_certificate: Some(mesh.client_certificate.clone()),
                })
            }),
            letsencrypt: letsencrypt.clone(),
        })
    }
}

fn merge_server_config(
    server: &ServerConfig<ConfigFileTypes>,
    cli: &Cli,
) -> Arc<ServerConfig<RuntimeTypes>> {
    let port = cli.port.or(server.port).unwrap_or(PORT);
    Arc::new(ServerConfig {
        host: {
            let host = cli.host.as_deref();
            let host = host.or(server.host.as_deref());
            host.unwrap_or_else(|| HOST).to_owned()
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
    })
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
) -> Option<Arc<MeshConfig<RuntimeTypes>>> {
    let mesh = mesh.as_ref();
    let client_name = cli.client_name.as_ref().cloned();
    let client_name = client_name.or(mesh.and_then(|m| m.client_name.to_owned()))?;
    let gateway_url = cli.gateway_url.as_ref().cloned();
    let gateway_url = gateway_url.or(mesh.and_then(|m| m.gateway_url.to_owned()))?;
    let gateway_pki = cli.gateway_pki.as_ref().cloned();
    let gateway_pki = gateway_pki.or(mesh.and_then(|m| m.gateway_pki.to_owned()));
    let client_certificate = cli.client_certificate.as_ref().cloned();
    let client_certificate = client_certificate
        .or(mesh.and_then(|m| m.client_certificate.to_owned()))
        .unwrap_or_else(|| {
            [&home(), ".terrazzo/client_certificate"]
                .iter()
                .collect::<PathBuf>()
                .to_string_lossy()
                .to_string()
        });
    Some(Arc::new(MeshConfig {
        client_name,
        gateway_url,
        gateway_pki,
        client_certificate,
    }))
}
