#![cfg(feature = "server")]

use std::path::Path;

use terrazzo::declare_asset;
use terrazzo::declare_assets_dir;
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
    declare_assets_dir!("wasm/snippets", "$CARGO_MANIFEST_DIR/assets/wasm/snippets");
}
