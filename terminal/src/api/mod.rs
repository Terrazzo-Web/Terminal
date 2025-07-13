pub mod client;
pub mod server;
pub mod shared;

#[cfg(feature = "server")]
use trz_gateway_common::id::ClientName;

#[cfg(all(feature = "client", not(feature = "server")))]
use self::client_name::ClientName;

pub mod client_address;
pub mod client_name;

#[cfg(feature = "correlation-id")]
const CORRELATION_ID: &str = "terrazzo-correlation-id";

#[cfg(feature = "terminal")]
const KEEPALIVE_TTL_HEADER: &str = "terrazzo-keepalive-ttl";

#[cfg(feature = "terminal")]
const NEWLINE: u8 = b'\n';

#[allow(unused)]
pub static APPLICATION_JSON: &str = "application/json";

#[test]
#[cfg(all(test, feature = "server"))]
fn application_json_test() {
    assert_eq!(APPLICATION_JSON, terrazzo::mime::APPLICATION_JSON);
}
