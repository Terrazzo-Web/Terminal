use std::ops::Deref;

use super::ClientName;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClientAddress(Vec<ClientName>);

impl Deref for ClientAddress {
    type Target = Vec<ClientName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ClientName> for ClientAddress {
    fn from(client_name: ClientName) -> Self {
        Self(vec![client_name])
    }
}

impl From<Vec<ClientName>> for ClientAddress {
    fn from(client_names: Vec<ClientName>) -> Self {
        Self(client_names)
    }
}

mod display {
    use std::fmt::Display;

    use super::ClientAddress;

    impl Display for ClientAddress {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0
                .iter()
                .map(|cn| cn.as_ref())
                .collect::<Vec<_>>()
                .join(" ≻ ")
                .fmt(f)
        }
    }
}
