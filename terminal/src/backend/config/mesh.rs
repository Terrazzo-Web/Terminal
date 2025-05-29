use std::ops::Deref;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::has_diff::DiffOption;

use super::types::ConfigTypes;
use super::types::RuntimeTypes;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeshConfig<T: ConfigTypes = RuntimeTypes> {
    /// The Client name.
    pub client_name: T::String,

    /// The Gateway endpoint.
    pub gateway_url: T::String,

    /// The Gateway CA.
    ///
    /// This is the Root CA of the Gateway server certificate.
    pub gateway_pki: T::MaybeString,

    /// The file to store the client certificate.
    pub client_certificate: T::String,

    /// The strategy to retry connecting.
    pub retry_strategy: T::RetryStrategy,
}

#[derive(Clone)]
pub struct DynamicMeshConfig(Arc<DynamicConfig<DiffOption<DiffArc<MeshConfig>>>>);

impl Deref for DynamicMeshConfig {
    type Target = Arc<DynamicConfig<DiffOption<DiffArc<MeshConfig>>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<DynamicConfig<DiffOption<DiffArc<MeshConfig>>>>> for DynamicMeshConfig {
    fn from(value: Arc<DynamicConfig<DiffOption<DiffArc<MeshConfig>>>>) -> Self {
        Self(value)
    }
}
