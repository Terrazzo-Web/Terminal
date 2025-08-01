use crate::api::client_address::ClientAddress;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortForward {
    pub id: i32,
    pub from: HostPortDefinition,
    pub to: HostPortDefinition,
}

impl Default for PortForward {
    fn default() -> Self {
        Self::new()
    }
}

impl PortForward {
    pub fn new() -> Self {
        use std::sync::atomic::AtomicI32;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicI32 = AtomicI32::new(0);

        let id = NEXT.fetch_add(1, SeqCst);
        Self {
            id,
            from: HostPortDefinition::default(),
            to: HostPortDefinition::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostPortDefinition {
    pub forwarded_remote: Option<ClientAddress>,
    pub host: String,
    pub port: u16,
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
