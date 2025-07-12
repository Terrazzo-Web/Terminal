use tonic::Status;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;

use crate::api::client_address::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::FilePath as FilePathProto;
use crate::text_editor::file_path::FilePath;

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

impl<B, F> From<FilePathProto> for FilePath<B, F>
where
    String: Into<B>,
    String: Into<F>,
{
    fn from(proto: FilePathProto) -> Self {
        Self {
            base: proto.base.into(),
            file: proto.file.into(),
        }
    }
}

impl<B, F> From<FilePath<B, F>> for FilePathProto
where
    B: ToString,
    F: ToString,
{
    fn from(proto: FilePath<B, F>) -> Self {
        Self {
            base: proto.base.to_string(),
            file: proto.file.to_string(),
        }
    }
}
