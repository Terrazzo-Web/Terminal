pub mod static_assets;

pub fn install_assets() {
    crate::declare_asset!("/assets/css/common.css")
        .mime(mime::TEXT_CSS_UTF_8.as_ref())
        .install();
}
