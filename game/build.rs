use std::env;
use std::path::PathBuf;

use scopeguard::defer;
use terrazzo_build::BuildOptions;

const SERVER_FEATURE: &str = "CARGO_FEATURE_SERVER";
const CLIENT_FEATURE: &str = "CARGO_FEATURE_CLIENT";
const MAX_LEVEL_INFO: &str = "CARGO_FEATURE_MAX_LEVEL_INFO";
const MAX_LEVEL_DEBUG: &str = "CARGO_FEATURE_MAX_LEVEL_DEBUG";
const DEBUG: &str = "CARGO_FEATURE_DEBUG";
const NO_WASM_BUILD: &str = "CARGO_FEATURE_NO_WASM_BUILD";

fn main() {
    if env::var("DOCS_RS") != Err(env::VarError::NotPresent) {
        return;
    }
    if env::var(NO_WASM_BUILD).is_ok() {
        return;
    };
    let Ok(server_feature) = env::var(SERVER_FEATURE) else {
        return;
    };
    unsafe { env::remove_var(SERVER_FEATURE) };
    defer!(unsafe { std::env::set_var(SERVER_FEATURE, server_feature) });

    if env::var(CLIENT_FEATURE).is_ok() {
        println!("cargo::warning=Can't enable both 'client' and 'server' features");
    }

    let cargo_manifest_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let server_dir = cargo_manifest_dir.join("target");
    std::fs::create_dir_all(server_dir.join("assets")).expect("server_dir");
    let client_dir: PathBuf = cargo_manifest_dir.clone();

    let mut wasm_pack_options = vec!["--no-default-features", "--features", "client"];
    if env::var(MAX_LEVEL_INFO).is_ok() {
        wasm_pack_options.extend(["--features", "max-level-info"]);
    }
    if env::var(MAX_LEVEL_DEBUG).is_ok() {
        wasm_pack_options.extend(["--features", "max-level-debug"]);
    }
    if env::var(DEBUG).is_ok() {
        wasm_pack_options.extend(["--features", "debug"]);
    }
    let wasm_pack_options = &wasm_pack_options;
    terrazzo_build::build(BuildOptions {
        client_dir,
        server_dir,
        wasm_pack_options,
    })
    .unwrap();

    terrazzo_build::build_css();
}
