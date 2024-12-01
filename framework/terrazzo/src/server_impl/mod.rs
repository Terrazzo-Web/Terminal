pub mod static_assets;

pub fn install_assets() {
    crate::declare_scss_asset!("target/css/common.scss").install();
}

pub use ::autoclone_macro::autoclone;
pub use ::axum;
pub use ::http;
pub use ::mime;
