use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use terrazzo::autoclone;
use tracing::Instrument;
use tracing::debug;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::has_diff::DiffOption;
use trz_gateway_common::dynamic_config::has_diff::HasDiff;
use trz_gateway_common::dynamic_config::mode::RO;
use trz_gateway_server::server::acme::DynamicAcmeConfig;

use super::Config;
use super::ConfigFile;
use super::ConfigImpl;
use super::DynConfig;
use super::mesh::DynamicMeshConfig;
use super::server::DynamicServerConfig;
use crate::backend::cli::Cli;
use crate::backend::config::server::ServerConfig;

impl Config {
    #[autoclone]
    pub fn into_dyn(self, cli: &Cli) -> DiffArc<DynConfig> {
        let config = Arc::from(DynamicConfig::from(DiffArc::from(self)));
        let server = DynamicServerConfig::from(config.derive(
            |config| config.server.clone(),
            |config, server_config| {
                if HasDiff::is_same(&config.server, server_config) {
                    return None;
                }
                debug!("Updated server config");
                Some(DiffArc::from(Config::from(ConfigImpl {
                    server: server_config.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let mesh = DynamicMeshConfig::from(config.derive(
            |config| config.mesh.clone(),
            |config, mesh_config| {
                if DiffOption::is_same(&config.mesh, mesh_config) {
                    return None;
                }
                debug!("Updated mesh config");
                Some(DiffArc::from(Config::from(ConfigImpl {
                    mesh: mesh_config.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let letsencrypt = DynamicAcmeConfig::from(config.derive(
            |config| config.letsencrypt.clone(),
            |config, letsencrypt| {
                if DiffOption::is_same(&config.letsencrypt, letsencrypt) {
                    return None;
                }
                debug!("Updated letsencrypt config");
                Some(DiffArc::from(Config::from(ConfigImpl {
                    letsencrypt: letsencrypt.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let config_file_path = cli.config_file.to_owned();
        let dyn_config_file: Arc<DynamicConfig<(), RO>> = config.view(move |config| {
            autoclone!(config_file_path);
            if let Some(config_file_path) = config_file_path.as_deref() {
                let () = config
                    .to_config_file()
                    .save(config_file_path)
                    .inspect(|()| info!("Saved config file {config_file_path}"))
                    .unwrap_or_else(|error| warn!("Failed to save {config_file_path}: {error}"));
            }
        });
        let config = DiffArc::from(DynConfig {
            server,
            mesh,
            letsencrypt,
            config,
            dyn_config_file,
        });
        if let Some(config_file_path) = &config_file_path {
            tokio::spawn(poll_config_file(
                config_file_path.to_owned(),
                config.clone(),
            ));
        }
        return config;
    }
}

async fn poll_config_file(config_file_path: String, config: DiffArc<DynConfig>) {
    let span = info_span!("Polling config file", config_file_path);
    async move {
        let mut last_modified = SystemTime::UNIX_EPOCH;
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let metadata = match std::fs::metadata(&config_file_path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    warn!("Failed to get file metadata: {error}");
                    continue;
                }
            };
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(error) => {
                    warn!("Failed to get modification timestamp: {error}");
                    continue;
                }
            };
            if modified == last_modified {
                continue;
            }
            last_modified = modified;
            let new_config_file = match ConfigFile::load(&config_file_path) {
                Ok(new_config_file) => new_config_file,
                Err(error) => {
                    warn!("Failed to load config file: {error}");
                    continue;
                }
            };
            let new = new_config_file.merge(&Cli::default());
            let is_changed = config.server.try_set(|old| {
                let mut result = Err(());
                fn get_or_init<'t>(
                    old: &ServerConfig,
                    result: &'t mut Result<ServerConfig, ()>,
                ) -> &'t mut ServerConfig {
                    match result {
                        Ok(r) => r,
                        Err(()) => {
                            *result = Ok(old.clone());
                            return result.as_mut().unwrap();
                        }
                    }
                }
                if new.server.password != old.password {
                    info!("Changed: password");
                    let result = get_or_init(old, &mut result);
                    result.password = new.server.password.clone();
                }
                if new.server.token_lifetime != old.token_lifetime {
                    info!("Changed: token_lifetime");
                    let result = get_or_init(old, &mut result);
                    result.token_lifetime = new.server.token_lifetime;
                }
                if new.server.token_refresh != old.token_refresh {
                    info!("Changed: token_refresh");
                    let result = get_or_init(old, &mut result);
                    result.token_refresh = new.server.token_refresh;
                }

                return result.map(DiffArc::from);
            });
            match is_changed {
                Ok(()) => debug!("ServerConfig has changed"),
                Err(()) => debug!("ServerConfig hasn't changed"),
            }
        }
    }
    .instrument(span)
    .await
}
