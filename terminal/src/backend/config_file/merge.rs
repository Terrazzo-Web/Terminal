use std::path::PathBuf;
use std::time::Duration;

use tracing::warn;

use super::ConfigFile;
use super::MeshConfig;
use super::ServerConfig;
use super::types::ConfigFileTypes;
use super::types::RuntimeTypes;
use crate::backend::HOST;
use crate::backend::PORT;
use crate::backend::auth::DEFAULT_TOKEN_LIFETIME;
use crate::backend::auth::DEFAULT_TOKEN_REFRESH;
use crate::backend::cli::Cli;
use crate::backend::home;

impl ConfigFile<ConfigFileTypes> {
    pub fn merge(self, cli: &Cli) -> ConfigFile<RuntimeTypes> {
        ConfigFile {
            server: merge_server_config(self.server, cli),
            mesh: merge_mesh_config(self.mesh, cli),
        }
    }
}

impl ConfigFile<RuntimeTypes> {
    pub fn to_config_file(&self) -> ConfigFile<ConfigFileTypes> {
        ConfigFile {
            server: ServerConfig {
                host: self.server.host.clone().into(),
                port: self.server.port.into(),
                pidfile: self.server.pidfile.clone().into(),
                private_root_ca: self.server.private_root_ca.clone().into(),
                password: self.server.password.clone(),
                token_lifetime: Some(
                    humantime::format_duration(self.server.token_lifetime).to_string(),
                ),
                token_refresh: Some(
                    humantime::format_duration(self.server.token_refresh).to_string(),
                ),
            },
            mesh: self.mesh.as_ref().map(|mesh| MeshConfig {
                client_name: mesh.client_name.clone().into(),
                gateway_url: mesh.gateway_url.clone().into(),
                gateway_pki: mesh.gateway_pki.clone(),
                client_certificate: mesh.client_certificate.clone().into(),
            }),
        }
    }
}

fn merge_server_config(
    server: ServerConfig<ConfigFileTypes>,
    cli: &Cli,
) -> ServerConfig<RuntimeTypes> {
    let host = cli.host.as_ref().cloned();
    let host = host.or(server.host).unwrap_or_else(|| HOST.to_owned());
    let port = cli.port.or(server.port).unwrap_or(PORT);
    let pidfile = cli.pidfile.as_ref().cloned();
    let pidfile = pidfile.or(server.pidfile).unwrap_or_else(|| {
        [home(), format!(".terrazzo/terminal-{port}.pid")]
            .iter()
            .collect::<PathBuf>()
            .to_string_lossy()
            .to_string()
    });
    let private_root_ca = cli.private_root_ca.as_ref().cloned();
    let private_root_ca = private_root_ca
        .or(server.private_root_ca)
        .unwrap_or_else(|| {
            [&home(), ".terrazzo/root_ca"]
                .iter()
                .collect::<PathBuf>()
                .to_string_lossy()
                .to_string()
        });
    let token_lifetime = parse_duration(server.token_lifetime).unwrap_or(DEFAULT_TOKEN_LIFETIME);
    let token_refresh = parse_duration(server.token_refresh).unwrap_or(DEFAULT_TOKEN_REFRESH);
    ServerConfig {
        host,
        port,
        pidfile,
        private_root_ca,
        password: server.password,
        token_lifetime,
        token_refresh,
    }
}

fn parse_duration(duration: Option<String>) -> Option<Duration> {
    duration.and_then(|duration| {
        humantime::parse_duration(&duration)
            .inspect_err(|error| warn!("Failed to parse '{duration}': {error}"))
            .ok()
    })
}

fn merge_mesh_config(
    mesh: Option<MeshConfig<ConfigFileTypes>>,
    cli: &Cli,
) -> Option<MeshConfig<RuntimeTypes>> {
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
    Some(MeshConfig {
        client_name,
        gateway_url,
        gateway_pki,
        client_certificate,
    })
}
