use std::ops::Deref;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_common::dynamic_config::DynamicConfig;

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
}

#[derive(Clone)]
pub struct DynamicMeshConfig(Arc<DynamicConfig<Option<Arc<MeshConfig>>>>);

impl Deref for DynamicMeshConfig {
    type Target = Arc<DynamicConfig<Option<Arc<MeshConfig>>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<DynamicConfig<Option<Arc<MeshConfig>>>>> for DynamicMeshConfig {
    fn from(value: Arc<DynamicConfig<Option<Arc<MeshConfig>>>>) -> Self {
        Self(value)
    }
}
