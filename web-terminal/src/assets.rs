use std::path::Path;

use include_directory::include_directory;
use include_directory::Dir;
use terrazzo::declare_asset;
use terrazzo::declare_scss_asset;
use terrazzo::static_assets::AssetBuilder;

pub fn install_assets() {
    terrazzo::install_assets();
    declare_asset!("/assets/index.html")
        .mime(terrazzo::mime::TEXT_HTML_UTF_8.as_ref())
        .install();
    declare_asset!("/assets/bootstrap.js").install();
    declare_asset!("/assets/images/favicon.ico").install();
    declare_scss_asset!("target/css/web-terminal.scss").install();
    install_icons();
    install_xterm();
    install_wasm();
}

fn install_icons() {
    fn install_icon(mut asset: AssetBuilder) {
        let path = Path::new("icons").join(asset.asset_name);
        let path = path.as_os_str().to_str().unwrap();
        asset.asset_name = path.into();
        asset.install();
    }
    install_icon(declare_asset!("/assets/icons/plus-square.svg"));
    install_icon(declare_asset!("/assets/icons/x-lg.svg"));
}

fn install_xterm() {
    declare_asset!("/assets/xterm/css/xterm.css").install();
    declare_asset!("/assets/xterm/lib/xterm.js").install();
    declare_asset!("/assets/xterm/lib/addon-fit.js")
        .asset_name("xterm-addon-fit.js")
        .install();
    declare_asset!("/assets/xterm/lib/addon-web-links.js")
        .asset_name("xterm-addon-web-links.js")
        .install();
}

fn install_wasm() {
    declare_asset!("/assets/wasm/web_terminal.js")
        .asset_name("wasm/web_terminal.js")
        .install();
    declare_asset!("/assets/wasm/web_terminal_bg.wasm")
        .asset_name("wasm/web_terminal_bg.wasm")
        .install();

    static DIR: Dir<'_> = include_directory!("$CARGO_MANIFEST_DIR/assets/wasm/snippets");
    install_wasm_snippets(&DIR);
}

fn install_wasm_snippets(dir: &Dir<'static>) {
    for entry in dir.entries() {
        if let Some(dir) = entry.as_dir() {
            install_wasm_snippets(dir);
        }
        if let Some(file) = entry.as_file() {
            let path = Path::new("wasm/snippets/").join(entry.path());
            let path = path.as_os_str().to_str().unwrap();
            AssetBuilder::new(path, file.contents())
                .asset_name(path)
                .install();
        }
    }
}
