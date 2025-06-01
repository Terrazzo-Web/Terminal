#![cfg(feature = "server")]

use terrazzo::declare_asset;
use terrazzo::declare_assets_dir;
use terrazzo::declare_scss_asset;

pub fn install_assets() {
    terrazzo::install_assets();
    declare_asset!("/assets/index.html")
        .mime(terrazzo::mime::TEXT_HTML_UTF_8.as_ref())
        .install();
    declare_asset!("/assets/bootstrap.js").install();
    declare_asset!("/assets/images/favicon.ico").install();
    declare_scss_asset!("target/css/terrazzo-terminal.scss").install();
    install_icons();
    install_xterm();
    install_wasm();
}

fn install_icons() {
    #[cfg(not(feature = "client"))]
    fn install_icon(mut asset: terrazzo::static_assets::AssetBuilder) {
        let path = std::path::Path::new("icons").join(asset.asset_name);
        let path = path.as_os_str().to_str().unwrap();
        asset.asset_name = path.into();
        asset.install();
    }

    #[cfg(feature = "client")]
    fn install_icon(_: &'static str) {}

    install_icon(super::icons::add_tab());
    install_icon(super::icons::close_tab());
    install_icon(super::icons::menu());
    install_icon(super::icons::terminal());
    install_icon(super::icons::text_editor());
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
    declare_asset!("/target/assets/wasm/terrazzo_terminal.js")
        .asset_name("wasm/terrazzo_terminal.js")
        .install();
    declare_asset!("/target/assets/wasm/terrazzo_terminal_bg.wasm")
        .asset_name("wasm/terrazzo_terminal_bg.wasm")
        .install();
    declare_assets_dir!(
        "wasm/snippets",
        "$CARGO_MANIFEST_DIR/target/assets/wasm/snippets"
    );
}
