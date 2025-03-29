use prost as _;
use prost_types as _;

pub mod terrazzo {
    pub mod gateway {
        pub mod client {
            use trz_gateway_common::id::ClientName;

            include!(concat!(env!("OUT_DIR"), "/terrazzo.gateway.client.rs"));

            impl ClientAddress {
                pub fn leaf(&self) -> Option<ClientName> {
                    self.via.last().map(|s| ClientName::from(s.as_str()))
                }
            }
        }
    }
}
