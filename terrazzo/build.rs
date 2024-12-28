use std::env;
use std::path::PathBuf;

use scopeguard::defer;
use terrazzo_build::BuildOptions;

const SERVER_FEATURE: &str = "CARGO_FEATURE_SERVER";
const CLIENT_FEATURE: &str = "CARGO_FEATURE_CLIENT";
const MAX_LEVEL_INFO: &str = "CARGO_FEATURE_MAX_LEVEL_INFO";
const MAX_LEVEL_DEBUG: &str = "CARGO_FEATURE_MAX_LEVEL_DEBUG";

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

    let server_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let client_dir: PathBuf = server_dir.clone();
    let mut wasm_pack_options = vec!["--no-default-features", "--features", "client"];
    if env::var(MAX_LEVEL_INFO).is_ok() {
        wasm_pack_options.extend(["--features", "max_level_info"]);
    }
    if env::var(MAX_LEVEL_DEBUG).is_ok() {
        wasm_pack_options.extend(["--features", "max_level_debug"]);
    }
    let wasm_pack_options = &wasm_pack_options;
    terrazzo_build::build(BuildOptions {
        client_dir,
        server_dir,
        wasm_pack_options,
    })
    .unwrap();

    terrazzo_build::build_css("terrazzo");
}
