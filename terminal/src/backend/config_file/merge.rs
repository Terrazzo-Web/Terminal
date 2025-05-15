use crate::backend::HOST;
use crate::backend::PORT;
use crate::backend::cli::Cli;

use super::ConfigFile;
use super::MeshConfig;
use super::ServerConfig;
use super::types::ConfigFileTypes;
use super::types::RuntimeTypes;

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
    let home = || std::env::var("HOME").expect("HOME");
    let host = cli.host.as_ref().cloned();
    let host = host.or(server.host).unwrap_or_else(|| HOST.to_owned());
    let port = cli.port.or(server.port).unwrap_or(PORT);
    let pidfile = cli.pidfile.as_ref().cloned();
    let pidfile = pidfile
        .or(server.pidfile)
        .unwrap_or_else(|| format!("{}/.terrazzo/terminal-{port}.pid", home()));
    let private_root_ca = cli.private_root_ca.as_ref().cloned();
    let private_root_ca = private_root_ca
        .or(server.private_root_ca)
        .unwrap_or_else(|| format!("{}/.terrazzo/root_ca", home()));
    let password = cli.password.as_ref().cloned();
    let password = password.or(server.password).unwrap_or_default();
    ServerConfig {
        host,
        port,
        pidfile,
        private_root_ca,
        password,
    }
}

fn merge_mesh_config(
    mesh: Option<MeshConfig<ConfigFileTypes>>,
    cli: &Cli,
) -> Option<MeshConfig<RuntimeTypes>> {
    let mesh = mesh.as_ref();
    let home = || std::env::var("HOME").expect("HOME");
    let client_name = cli.client_name.as_ref().cloned();
    let client_name = client_name.or(mesh.and_then(|m| m.client_name.to_owned()))?;
    let gateway_url = cli.gateway_url.as_ref().cloned();
    let gateway_url = gateway_url.or(mesh.and_then(|m| m.gateway_url.to_owned()))?;
    let gateway_pki = cli.gateway_pki.as_ref().cloned();
    let gateway_pki = gateway_pki.or(mesh.and_then(|m| m.gateway_pki.to_owned()).flatten());
    let client_certificate = cli.client_certificate.as_ref().cloned();
    let client_certificate = client_certificate
        .or(mesh.and_then(|m| m.client_certificate.to_owned()))
        .unwrap_or_else(|| format!("{}/.terrazzo/client_certificate", home()));
    Some(MeshConfig {
        client_name,
        gateway_url,
        gateway_pki,
        client_certificate,
    })
}
