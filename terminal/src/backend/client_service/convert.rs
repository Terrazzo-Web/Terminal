use tonic::Status;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;

use crate::api::client_address::ClientAddress;
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;

impl From<ClientAddressProto> for ClientAddress {
    fn from(proto: ClientAddressProto) -> Self {
        proto
            .via
            .into_iter()
            .map(ClientName::from)
            .collect::<Vec<_>>()
            .into()
    }
}

impl ClientAddressProto {
    pub fn of(client_address: &[impl AsRef<str>]) -> Self {
        Self {
            via: client_address
                .iter()
                .map(|x| x.as_ref().to_owned())
                .collect(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Impossible {}

impl From<Impossible> for Status {
    fn from(_: Impossible) -> Self {
        unreachable!()
    }
}

impl IsHttpError for Impossible {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        unreachable!()
    }
}
