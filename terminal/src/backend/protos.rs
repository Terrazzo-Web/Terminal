use prost as _;
use prost_types as _;

pub mod terrazzo {
    pub mod gateway {
        pub mod client {
            use trz_gateway_common::id::ClientName;

            include!(concat!(env!("OUT_DIR"), "/terrazzo.gateway.client.rs"));

            impl ClientAddress {
                pub fn leaf(&self) -> ClientName {
                    self.via
                        .last()
                        .expect("ClientAddress leaf")
                        .to_owned()
                        .into()
                }
            }
        }
    }
}
