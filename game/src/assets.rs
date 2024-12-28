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
    declare_scss_asset!("target/css/game.scss").install();
    declare_assets_dir!("game", "$CARGO_MANIFEST_DIR/assets/game");
    install_wasm();
}

fn install_wasm() {
    declare_asset!("/assets/wasm/game.js")
        .asset_name("wasm/game.js")
        .install();
    declare_asset!("/assets/wasm/game_bg.wasm")
        .asset_name("wasm/game_bg.wasm")
        .install();
}
