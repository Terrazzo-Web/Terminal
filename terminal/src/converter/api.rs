use std::sync::Arc;

use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

#[server(protocol = Http<Json, Json>)]
pub async fn get_conversions(
    remote: Option<ClientAddress>,
    input: String,
) -> Result<Conversions, ServerFnError> {
    Ok(super::service::GET_CONVERSIONS_FN
        .call(remote.unwrap_or_default(), ConversionsRequest { input })
        .await?)
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct ConversionsRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub input: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Conversions {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub conversions: Arc<Vec<Conversion>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConversionImpl {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "l"))]
    pub language: Language,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub content: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Conversion(Arc<ConversionImpl>);

impl Conversion {
    #[cfg(feature = "server")]
    pub fn new(language: Language, content: String) -> Self {
        Self(Arc::new(ConversionImpl { language, content }))
    }
}

impl std::ops::Deref for Conversion {
    type Target = ConversionImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for Conversion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversion")
            .field("language", &self.language)
            .finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
// serde
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Language {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "n"))]
    pub name: Arc<str>,
}

impl Language {
    #[cfg(feature = "server")]
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self { name: name.into() }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.name, f)
    }
}
