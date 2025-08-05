use std::sync::Arc;

use nameth::NamedType as _;
use nameth::nameth;
use serde::Deserialize;
use serde::Serialize;

#[nameth]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientName {
    id: Arc<str>,
}

impl From<String> for ClientName {
    fn from(id: String) -> Self {
        Self {
            id: id.into_boxed_str().into(),
        }
    }
}

impl From<&str> for ClientName {
    fn from(id: &str) -> Self {
        id.to_owned().into()
    }
}

impl std::fmt::Display for ClientName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

impl AsRef<str> for ClientName {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for ClientName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(ClientName::type_name())
            .field(&self.id.to_string())
            .finish()
    }
}
