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
    declare_scss_asset!("target/css/game.scss").install();

    install_game();
    install_wasm();
}

fn install_game() {
    static DIR: Dir<'_> = include_directory!("$CARGO_MANIFEST_DIR/assets/game");
    install_wasm_snippets(&DIR);
}

fn install_wasm_snippets(dir: &Dir<'static>) {
    for entry in dir.entries() {
        if let Some(dir) = entry.as_dir() {
            install_wasm_snippets(dir);
        }
        if let Some(file) = entry.as_file() {
            let path = Path::new("game/").join(entry.path());
            let path = path.as_os_str().to_str().unwrap();
            AssetBuilder::new(path, file.contents())
                .asset_name(path)
                .install();
        }
    }
}

fn install_wasm() {
    declare_asset!("/assets/wasm/game.js")
        .asset_name("wasm/game.js")
        .install();
    declare_asset!("/assets/wasm/game_bg.wasm")
        .asset_name("wasm/game_bg.wasm")
        .install();
}
