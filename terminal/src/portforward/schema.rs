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
