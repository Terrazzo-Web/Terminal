#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub mod shared;

#[cfg(feature = "server")]
use trz_gateway_common::id::ClientName;

#[cfg(all(feature = "client", not(feature = "server")))]
use self::client_name::ClientName;

pub mod client_address;
pub mod client_name;

const CORRELATION_ID: &str = "terrazzo-correlation-id";
const KEEPALIVE_TTL_HEADER: &str = "terrazzo-keepalive-ttl";

const NEWLINE: u8 = b'\n';

pub const STREAMING_WINDOW_SIZE: usize = 200 * 1000;

#[allow(unused)]
pub static APPLICATION_JSON: &str = "application/json";

#[test]
#[cfg(all(test, feature = "server"))]
fn application_json_test() {
    assert_eq!(APPLICATION_JSON, terrazzo::mime::APPLICATION_JSON);
}
