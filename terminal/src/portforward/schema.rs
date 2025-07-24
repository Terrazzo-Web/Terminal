use crate::api::client_address::ClientAddress;

#[derive(Clone, Debug)]
pub struct PortForward {
    pub id: i32,
    pub from: HostPortDefinition,
    pub to: HostPortDefinition,
}

impl PortForward {
    pub fn new(from: HostPortDefinition, to: HostPortDefinition) -> Self {
        use std::sync::atomic::AtomicI32;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicI32 = AtomicI32::new(0);

        let id = NEXT.fetch_add(1, SeqCst);
        Self { id, from, to }
    }
}

#[derive(Clone, Debug)]
pub struct HostPortDefinition {
    pub remote: Option<ClientAddress>,
    pub host: String,
    pub port: u16,
}
