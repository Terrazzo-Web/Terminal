use std::env;

use scopeguard::defer;

const SERVER_FEATURE: &str = "CARGO_FEATURE_SERVER";
const CLIENT_FEATURE: &str = "CARGO_FEATURE_CLIENT";

fn main() {
    let Ok(server_feature) = env::var(SERVER_FEATURE) else {
        return;
    };
    env::remove_var(SERVER_FEATURE);
    defer!(std::env::set_var(SERVER_FEATURE, server_feature));

    if env::var(CLIENT_FEATURE).is_ok() {
        println!("cargo::warning=Can't enable both 'client' and 'server' features");
        return;
    }

    terrazzo_build::build_css();
}
