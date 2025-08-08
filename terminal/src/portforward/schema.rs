use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use crate::api::client_address::ClientAddress;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortForward {
    pub id: i32,
    pub from: HostPortDefinition,
    pub to: HostPortDefinition,
    pub state: PortForwardState,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PortForwardState(Arc<Mutex<PortForwardStateImpl>>);

impl PortForwardState {
    pub fn lock(&self) -> MutexGuard<PortForwardStateImpl> {
        self.0.lock().expect("PortForwardState")
    }
}

impl PartialEq for PortForwardState {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for PortForwardState {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PortForwardStateImpl {
    pub count: i32,
    pub status: PortForwardStatus,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum PortForwardStatus {
    #[default]
    Pending,
    Up,
    Failed(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct HostPortDefinition(Arc<HostPortDefinitionImpl>);

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostPortDefinitionImpl {
    pub forwarded_remote: Option<ClientAddress>,
    pub host: String,
    pub port: u16,
}

impl HostPortDefinition {
    pub fn new(forwarded_remote: Option<ClientAddress>, host: String, port: u16) -> Self {
        Self(Arc::new(HostPortDefinitionImpl {
            forwarded_remote,
            host,
            port,
        }))
    }
}

impl Deref for HostPortDefinition {
    type Target = HostPortDefinitionImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for HostPortDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let forwarded_remote = self
            .forwarded_remote
            .as_ref()
            .filter(|remote| !remote.is_empty())
            .map(|remote| remote.to_string())
            .unwrap_or_else(|| "Local".to_string());
        let host = &self.host;
        let port = self.port;
        write!(f, "{forwarded_remote}:{host}:{port}")
    }
}
