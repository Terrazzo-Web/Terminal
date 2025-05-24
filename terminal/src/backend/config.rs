use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::mode::RO;
use trz_gateway_server::server::acme::AcmeConfig;
use trz_gateway_server::server::acme::DynamicAcmeConfig;

use self::mesh::DynamicMeshConfig;
use self::mesh::MeshConfig;
use self::server::DynamicServerConfig;
use self::server::ServerConfig;
use self::types::ConfigFileTypes;
use self::types::ConfigTypes;
use self::types::RuntimeTypes;

mod into_dyn;
pub(in crate::backend) mod io;
pub(in crate::backend) mod kill;
mod merge;
pub mod mesh;
pub(in crate::backend) mod password;
pub(in crate::backend) mod pidfile;
pub mod server;
pub(in crate::backend) mod types;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigFile(ConfigImpl<ConfigFileTypes>);

impl Deref for ConfigFile {
    type Target = ConfigImpl<ConfigFileTypes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct DynConfig {
    config: Arc<DynamicConfig<Arc<Config>>>,
    pub server: DynamicServerConfig,
    pub mesh: DynamicMeshConfig,
    pub letsencrypt: DynamicAcmeConfig,

    #[expect(unused)]
    dyn_config_file: Arc<DynamicConfig<(), RO>>,
}

impl Deref for DynConfig {
    type Target = Arc<DynamicConfig<Arc<Config>>>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config(ConfigImpl<RuntimeTypes>);

impl Deref for Config {
    type Target = ConfigImpl<RuntimeTypes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ConfigImpl<RuntimeTypes>> for Config {
    fn from(value: ConfigImpl<RuntimeTypes>) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigImpl<T: ConfigTypes> {
    #[serde(default)]
    pub server: Arc<ServerConfig<T>>,
    pub mesh: Option<Arc<MeshConfig<T>>>,
    pub letsencrypt: Option<Arc<AcmeConfig>>,
}
