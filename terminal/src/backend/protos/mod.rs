use prost as _;
use prost_types as _;

pub mod terrazzo {
    pub mod notify {
        include!(concat!(env!("OUT_DIR"), "/terrazzo.notify.rs"));
    }
    pub mod remotefn {
        include!(concat!(env!("OUT_DIR"), "/terrazzo.remotefn.rs"));
    }
    pub mod shared {
        include!(concat!(env!("OUT_DIR"), "/terrazzo.shared.rs"));
        use trz_gateway_common::id::ClientName;

        impl ClientAddress {
            pub fn leaf(&self) -> Option<ClientName> {
                self.via.last().map(|s| ClientName::from(s.as_str()))
            }
        }
    }
    pub mod terminal {
        include!(concat!(env!("OUT_DIR"), "/terrazzo.terminal.rs"));
    }
}
