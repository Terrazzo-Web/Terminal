use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use tracing::Instrument;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::mode::RO;
use trz_gateway_server::server::acme::DynamicAcmeConfig;

use super::Config;
use super::ConfigFile;
use super::ConfigImpl;
use super::DynConfig;
use super::mesh::DynamicMeshConfig;
use super::server::DynamicServerConfig;
use super::types::RuntimeTypes;
use crate::backend::cli::Cli;
use crate::backend::config::server::ServerConfig;

impl Config {
    pub fn into_dyn(self, cli: &Cli) -> Arc<DynConfig> {
        let config = Arc::new(DynamicConfig::from(Arc::new(self)));
        let server = DynamicServerConfig::from(config.derive(
            |config| config.server.clone(),
            |config, server_config| {
                if Arc::ptr_eq(&config.server, server_config) {
                    return None;
                }
                Some(Arc::new(Config::from(ConfigImpl {
                    server: server_config.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let mesh = DynamicMeshConfig::from(config.derive(
            |config| config.mesh.clone(),
            |config, mesh_config| {
                if option_ptr_eq(&config.mesh, mesh_config) {
                    return None;
                }
                Some(Arc::new(Config::from(ConfigImpl {
                    mesh: mesh_config.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let letsencrypt = DynamicAcmeConfig::from(config.derive(
            |config| config.letsencrypt.clone(),
            |config, letsencrypt| {
                if option_ptr_eq(&config.letsencrypt, letsencrypt) {
                    return None;
                }
                Some(Arc::new(Config::from(ConfigImpl {
                    letsencrypt: letsencrypt.clone(),
                    ..(**config).clone()
                })))
            },
        ));
        let config_file_path = cli.config_file.to_owned();
        if let Some(config_file_path) = &config_file_path {
            tokio::spawn(poll_config_file(
                config_file_path.to_owned(),
                config.clone(),
            ));
        }
        let dyn_config_file: Arc<DynamicConfig<(), RO>> = config.view(move |config| {
            if let Some(config_file_path) = config_file_path.as_deref() {
                let () = config
                    .to_config_file()
                    .save(config_file_path)
                    .inspect(|()| info!("Saved config file {config_file_path}"))
                    .unwrap_or_else(|error| warn!("Failed to save {config_file_path}: {error}"));
            }
        });
        Arc::new(DynConfig {
            server,
            mesh,
            letsencrypt,
            config,
            dyn_config_file,
        })
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

async fn poll_config_file(config_file_path: String, config: Arc<DynamicConfig<Arc<Config>>>) {
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
            let _ = config.try_set(|old| {
                let mut result = Err(());
                fn get_or_init(
                    result: &mut Result<ConfigImpl<RuntimeTypes>, ()>,
                ) -> &mut ConfigImpl<RuntimeTypes> {
                    match result {
                        Ok(r) => r,
                        Err(()) => {
                            *result = Ok(ConfigImpl::default());
                            return result.as_mut().unwrap();
                        }
                    }
                }
                if new.server.password != old.server.password {
                    info!("Changed: password");
                    let result = get_or_init(&mut result);
                    result.server = Arc::new(ServerConfig {
                        password: new.server.password.clone(),
                        ..(*old.server).clone()
                    });
                }
                if new.server.token_lifetime != old.server.token_lifetime {
                    info!("Changed: token_lifetime");
                    let result = get_or_init(&mut result);
                    result.server = Arc::new(ServerConfig {
                        token_lifetime: new.server.token_lifetime,
                        ..(*old.server).clone()
                    });
                }
                if new.server.token_refresh != old.server.token_refresh {
                    info!("Changed: token_refresh");
                    let result = get_or_init(&mut result);
                    result.server = Arc::new(ServerConfig {
                        token_refresh: new.server.token_refresh,
                        ..(*old.server).clone()
                    });
                }

                return result.map(Config::from).map(Arc::new);
            });
        }
    }
    .instrument(span)
    .await
}
