use std::sync::Arc;

use named::named;
use named::NamedType;
use serde::Deserialize;
use serde::Serialize;

#[named]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TerminalId {
    id: Arc<str>,
}

impl From<String> for TerminalId {
    fn from(id: String) -> Self {
        Self {
            id: id.into_boxed_str().into(),
        }
    }
}

impl From<&str> for TerminalId {
    fn from(id: &str) -> Self {
        id.to_owned().into()
    }
}

#[cfg(feature = "client")]
mod client {
    use terrazzo_client::prelude::XString;

    use super::TerminalId;

    impl From<XString> for TerminalId {
        fn from(value: XString) -> Self {
            match value {
                XString::Str(str) => str.into(),
                XString::Arc(arc) => Self { id: arc },
            }
        }
    }

    impl From<TerminalId> for XString {
        fn from(value: TerminalId) -> Self {
            XString::Arc(value.id)
        }
    }
}

impl std::fmt::Display for TerminalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

#[cfg(feature = "client")]
impl TerminalId {
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for TerminalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(TerminalId::type_name())
            .field(&self.id.to_string())
            .finish()
    }
}
