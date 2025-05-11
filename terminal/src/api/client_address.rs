use std::ops::Deref;

use super::ClientName;

#[cfg(all(feature = "client", not(feature = "server")))]
type Ptr<T> = std::rc::Rc<T>;

#[cfg(feature = "server")]
type Ptr<T> = std::sync::Arc<T>;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClientAddress(Ptr<Vec<ClientName>>);

impl Deref for ClientAddress {
    type Target = Vec<ClientName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ClientName> for ClientAddress {
    fn from(client_name: ClientName) -> Self {
        Self(vec![client_name].into())
    }
}

impl From<Vec<ClientName>> for ClientAddress {
    fn from(client_names: Vec<ClientName>) -> Self {
        Self(client_names.into())
    }
}

mod display {
    use std::fmt::Display;

    impl Display for super::ClientAddress {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut list = self.0.iter().map(|cn| cn.as_ref()).collect::<Vec<_>>();
            list.reverse();
            list.join(" ≻ ").fmt(f)
        }
    }
}
