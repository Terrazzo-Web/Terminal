use prost as _;
use prost_types as _;

pub mod terrazzo {
    pub mod gateway {
        pub mod client {
            include!(concat!(env!("OUT_DIR"), "/terrazzo.gateway.client.rs"));
        }
    }
}
