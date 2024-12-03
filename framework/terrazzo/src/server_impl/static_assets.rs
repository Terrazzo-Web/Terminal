use std::collections::HashMap;
use std::ffi::OsStr;
use std::future::ready;
use std::path::Path;
use std::sync::RwLock;

use axum::body::Body;
use axum::body::Bytes;
use axum::response::Response;
use http::header;
use http::HeaderValue;
use include_directory::Dir;
use tracing::debug;
use tracing::warn;

static ASSETS: RwLock<Option<HashMap<String, Asset>>> = RwLock::new(None);

#[must_use]
pub struct AssetBuilder {
    pub asset_name: String,
    file_name: String,
    mime: Option<HeaderValue>,
    content: &'static [u8],
}

impl AssetBuilder {
    pub fn new(name: impl Into<String>, content: &'static [u8]) -> Self {
        let file_name = name.into();
        let asset_name = Path::new(&file_name).file_name().unwrap();
        let asset_name = asset_name.to_str().unwrap().to_owned();
        Self {
            asset_name,
            file_name,
            mime: None,
            content,
        }
    }

    pub fn asset_name(self, asset_name: impl Into<String>) -> Self {
        Self {
            asset_name: asset_name.into(),
            ..self
        }
    }

    pub fn extension(self, extension: impl AsRef<OsStr>) -> Self {
        Self {
            asset_name: Path::new(&self.asset_name)
                .with_extension(extension)
                .to_str()
                .unwrap()
                .to_owned(),
            ..self
        }
    }

    pub fn mime<M>(self, mime: M) -> Self
    where
        HeaderValue: TryFrom<M>,
        <HeaderValue as TryFrom<M>>::Error: Into<http::Error>,
    {
        let mime = Some(mime.try_into().map_err(Into::into).unwrap());
        Self { mime, ..self }
    }

    pub fn install(self) {
        let mime = if let Some(mime) = self.mime {
            mime
        } else {
            mime_guess::from_path(self.file_name)
                .first_raw()
                .map(HeaderValue::from_static)
                .unwrap_or_else(|| {
                    HeaderValue::from_str(mime::APPLICATION_OCTET_STREAM.as_ref()).unwrap()
                })
        };
        add(
            self.asset_name,
            Asset {
                mime,
                content: self.content,
            },
        );
    }
}

#[macro_export]
macro_rules! declare_asset {
    ($file:expr $(,)?) => {
        $crate::static_assets::AssetBuilder::new(
            $file,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), $file)),
        )
    };
}

#[macro_export]
macro_rules! declare_scss_asset {
    ($file:expr $(,)?) => {
        $crate::static_assets::AssetBuilder::new(
            $file,
            $crate::static_assets::__macro_support::include_scss!($file).as_bytes(),
        )
        .mime($crate::mime::TEXT_CSS_UTF_8.as_ref())
        .extension("css")
    };
}

#[doc(hidden)]
pub mod __macro_support {
    pub use ::include_directory;
    pub use ::rsass_macros::include_scss;
}

struct Asset {
    mime: HeaderValue,
    content: &'static [u8],
}

fn add(name: String, value: Asset) {
    let mut assets = ASSETS.write().unwrap();
    if let Some(assets) = &mut *assets {
        let old = assets.insert(name.clone(), value);
        assert!(old.is_none(), "Duplicate asset '{name}'");
        return;
    }

    let mut map = HashMap::new();
    map.insert(name, value);
    *assets = Some(map);
}

pub fn get(path: &str) -> std::future::Ready<Response<Body>> {
    debug!("Getting {path}");
    let assets = ASSETS.read().expect(path);
    let assets = &*assets;
    let Some(asset) = assets.as_ref().and_then(|assets| assets.get(path)) else {
        warn!("Not found: {path}");
        return ready(Response::builder().status(404).body(Body::empty()).unwrap());
    };
    ready(
        Response::builder()
            .header(header::CONTENT_TYPE, asset.mime.clone())
            .header(header::CONTENT_LENGTH, asset.content.len().to_string())
            .body(Body::from(Bytes::from_static(asset.content)))
            .expect(path),
    )
}

#[macro_export]
macro_rules! declare_assets_dir {
    ($prefix:literal, $dir:tt) => {{
        use $crate::static_assets::__macro_support::include_directory;
        static DIR: include_directory::Dir<'_> = include_directory::include_directory!($dir);
        $crate::static_assets::install_dir($prefix, &DIR);
    }};
}

pub fn install_dir(prefix: &str, dir: &Dir<'static>) {
    for entry in dir.entries() {
        if let Some(dir) = entry.as_dir() {
            install_dir(prefix, dir);
        }
        if let Some(file) = entry.as_file() {
            let path = Path::new(prefix).join(entry.path());
            let path = path.as_os_str().to_str().unwrap();
            AssetBuilder::new(path, file.contents())
                .asset_name(path)
                .install();
        }
    }
}
