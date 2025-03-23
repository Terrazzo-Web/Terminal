use std::ops::Deref;
use std::ops::DerefMut;

use super::ClientName;

pub struct ClientAddress(Vec<ClientName>);

impl Deref for ClientAddress {
    type Target = Vec<ClientName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ClientAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
