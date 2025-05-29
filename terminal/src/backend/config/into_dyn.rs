use std::sync::Arc;

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
use trz_gateway_server::server::acme::AcmeConfig;
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
    let mut retry_strategy = config.with(|config| config.server.config_file_poll_strategy.clone());
    async move {
        let mut last_modified = None;
        loop {
            retry_strategy.wait().await;
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
            let last_modified = scopeguard::guard(&mut last_modified, |v| *v = Some(modified));
            if last_modified.is_none() || **last_modified == Some(modified) {
                continue;
            };

            debug!("Config file timestamp has changed");

            let new_config_file = match ConfigFile::load(&config_file_path) {
                Ok(new_config_file) => new_config_file,
                Err(error) => {
                    warn!("Failed to load config file: {error}");
                    continue;
                }
            };
            let new = new_config_file.merge(&Cli::default());
            retry_strategy = new.server.config_file_poll_strategy.clone();
            apply_server_config(&config, &new.server);
            apply_letsencrypt_config(&config, &new.letsencrypt);
        }
    }
    .instrument(span)
    .await
}

fn apply_server_config(config: &DiffArc<DynConfig>, new: &ServerConfig) {
    let is_server_changed = config.server.try_set(|old| {
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
        if new.password != old.password {
            info!("Changed: password");
            let result = get_or_init(old, &mut result);
            result.password = new.password.clone();
        }
        if new.token_lifetime != old.token_lifetime {
            info!("Changed: token_lifetime");
            let result = get_or_init(old, &mut result);
            result.token_lifetime = new.token_lifetime;
        }
        if new.token_refresh != old.token_refresh {
            info!("Changed: token_refresh");
            let result = get_or_init(old, &mut result);
            result.token_refresh = new.token_refresh;
        }

        return result.map(DiffArc::from);
    });
    match is_server_changed {
        Ok(()) => info!("ServerConfig has changed"),
        Err(()) => debug!("ServerConfig hasn't changed"),
    }
}

fn apply_letsencrypt_config(config: &DiffArc<DynConfig>, new: &DiffOption<DiffArc<AcmeConfig>>) {
    let is_letsencrypt_changed =
        config
            .letsencrypt
            .try_set(|old: &DiffOption<DiffArc<AcmeConfig>>| {
                match (old.as_deref(), new.as_deref()) {
                    (None, None) => false,
                    (None, Some(_)) | (Some(_), None) => true,
                    (Some(old), Some(new)) => old != new,
                }
                .then(|| new.clone())
                .ok_or(())
            });
    match is_letsencrypt_changed {
        Ok(()) => info!("Let's Encrypt config has changed"),
        Err(()) => debug!("Let's Encrypt config hasn't changed"),
    }
}
