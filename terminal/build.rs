use std::env;
use std::path::PathBuf;

use heck::ToKebabCase as _;
use heck::ToShoutySnakeCase as _;
use terrazzo_build::BuildOptions;

#[derive(Clone, Copy, Debug)]
enum Feature {
    DocsRs,
    Client,
    Server,
    MaxLevelDebug,
    MaxLevelInfo,
    Diagnostics,
    NoWasmBuild,
    Debug,

    Terminal,
    TerminalClient,
    TerminalServer,

    TextEditor,
    TextEditorClient,
    TextEditorServer,

    Converter,
    ConverterClient,
    ConverterServer,

    PortForward,
    PortForwardClient,
    PortForwardServer,
}

impl Feature {
    fn is_set(self) -> bool {
        std::env::var(self.env_name()).is_ok()
    }

    fn feature_name(self) -> String {
        format!("{self:?}").to_kebab_case()
    }

    fn env_name(self) -> String {
        format!("CargoFeature{self:?}").to_shouty_snake_case()
    }

    fn disable(self) -> Option<impl Drop> {
        let env_name = self.env_name();
        let value = std::env::var(&env_name).ok()?;
        unsafe { env::remove_var(&env_name) };
        Some(scopeguard::guard((), |_| unsafe {
            std::env::set_var(env_name, value)
        }))
    }

    fn propagate(self, wasm_pack_options: &mut Vec<String>) {
        if self.is_set() {
            self.add(wasm_pack_options);
        }
    }

    fn add(self, wasm_pack_options: &mut Vec<String>) {
        wasm_pack_options.extend(["--features".into(), self.feature_name()]);
    }
}

fn main() {
    build_client();
    build_protos();
}

fn build_client() {
    if Feature::NoWasmBuild.is_set() {
        return;
    }
    if Feature::DocsRs.is_set() {
        return;
    }

    let Some(disable_server_feature) = Feature::Server.disable() else {
        return;
    };
    let disable_server_features = (
        Feature::TerminalServer.disable(),
        Feature::TextEditorServer.disable(),
        Feature::ConverterServer.disable(),
        Feature::PortForwardServer.disable(),
    );

    if Feature::Client.is_set() {
        println!("cargo::warning=Can't enable both 'client' and 'server' features");
    }

    let cargo_manifest_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let server_dir = cargo_manifest_dir.join("target");
    std::fs::create_dir_all(server_dir.join("assets")).expect("server_dir");
    let client_dir: PathBuf = cargo_manifest_dir.clone();

    let mut wasm_pack_options = vec!["--no-default-features".into()];
    Feature::MaxLevelDebug.propagate(&mut wasm_pack_options);
    Feature::MaxLevelInfo.propagate(&mut wasm_pack_options);
    Feature::Diagnostics.propagate(&mut wasm_pack_options);
    Feature::Debug.propagate(&mut wasm_pack_options);
    if Feature::Terminal.is_set() {
        Feature::TerminalClient.add(&mut wasm_pack_options);
    }
    if Feature::TextEditor.is_set() {
        Feature::TextEditorClient.add(&mut wasm_pack_options);
    }
    if Feature::Converter.is_set() {
        Feature::ConverterClient.add(&mut wasm_pack_options);
    }
    if Feature::PortForward.is_set() {
        Feature::PortForwardClient.add(&mut wasm_pack_options);
    }
    let wasm_pack_options = wasm_pack_options
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    let wasm_pack_options = &wasm_pack_options;
    terrazzo_build::build(BuildOptions {
        client_dir,
        server_dir,
        wasm_pack_options,
    })
    .unwrap();
    terrazzo_build::build_css();

    drop(disable_server_feature);
    drop(disable_server_features);
}

fn build_protos() {
    if !Feature::Server.is_set() {
        return;
    };
    tonic_build::configure()
        .bytes([".terrazzo.terminal.LeaseItem.data"])
        .compile_protos(
            &[
                "src/backend/protos/notify.proto",
                "src/backend/protos/remote_fn.proto",
                "src/backend/protos/shared.proto",
                "src/backend/protos/terminal.proto",
            ],
            &["src/backend/protos/"],
        )
        .unwrap();
}
