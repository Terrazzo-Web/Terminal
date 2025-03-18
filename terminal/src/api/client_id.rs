use std::sync::Arc;

use nameth::NamedType;
use nameth::nameth;
use serde::Deserialize;
use serde::Serialize;

#[nameth]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientId {
    id: Arc<str>,
}

impl From<String> for ClientId {
    fn from(id: String) -> Self {
        Self {
            id: id.into_boxed_str().into(),
        }
    }
}

impl From<&str> for ClientId {
    fn from(id: &str) -> Self {
        id.to_owned().into()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

impl AsRef<str> for ClientId {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(ClientId::type_name())
            .field(&self.id.to_string())
            .finish()
    }
}
