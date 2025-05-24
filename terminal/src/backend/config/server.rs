use std::ops::Deref;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;

use super::types::ConfigTypes;
use super::types::Password;
use super::types::RuntimeTypes;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerConfig<T: ConfigTypes = RuntimeTypes> {
    /// The TCP host to listen to.
    pub host: T::String,

    /// The TCP port to listen to.
    pub port: T::Port,

    /// The file to store the pid of the daemon while it is running,
    pub pidfile: T::String,

    /// The file to the store private Root CA.
    pub private_root_ca: T::String,

    /// The password to login to the UI.
    pub password: Option<Password>,
    pub token_lifetime: T::Duration,
    pub token_refresh: T::Duration,
}

#[derive(Clone)]
pub struct DynamicServerConfig(pub(super) Arc<DynamicConfig<DiffArc<ServerConfig>>>);

impl Deref for DynamicServerConfig {
    type Target = Arc<DynamicConfig<DiffArc<ServerConfig>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<DynamicConfig<DiffArc<ServerConfig>>>> for DynamicServerConfig {
    fn from(value: Arc<DynamicConfig<DiffArc<ServerConfig>>>) -> Self {
        Self(value)
    }
}
