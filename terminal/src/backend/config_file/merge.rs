use std::path::PathBuf;

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

    ServerConfig {
        host,
        port,
        pidfile,
        private_root_ca,
        password: server.password,
        token_cookie_lifetime: server
            .token_cookie_lifetime
            .unwrap_or(DEFAULT_TOKEN_LIFETIME),
        token_cookie_refresh: server
            .token_cookie_lifetime
            .unwrap_or(DEFAULT_TOKEN_REFRESH),
    }
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
