use tonic::Status;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_common::id::ClientName;

use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalRequest;
use crate::api::TabTitle;
use crate::api::TerminalAddress;
use crate::api::TerminalDef;
use crate::api::client_address::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::FilePath as FilePathProto;
use crate::backend::protos::terrazzo::gateway::client::MaybeString;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalRequest as RegisterTerminalRequestProto;
use crate::backend::protos::terrazzo::gateway::client::TerminalAddress as TerminalAddressProto;
use crate::backend::protos::terrazzo::gateway::client::TerminalDef as TerminalDefProto;
use crate::backend::protos::terrazzo::gateway::client::register_terminal_request::RegisterTerminalMode as RegisterTerminalModeProto;
use crate::text_editor::file_path::FilePath;

impl From<TerminalDefProto> for TerminalDef {
    fn from(proto: TerminalDefProto) -> Self {
        Self {
            address: proto.address.unwrap_or_default().into(),
            title: TabTitle {
                shell_title: proto.shell_title,
                override_title: proto.override_title.map(|s| s.s),
            },
            order: proto.order,
        }
    }
}

impl From<TerminalDef> for TerminalDefProto {
    fn from(terminal_def: TerminalDef) -> Self {
        Self {
            address: Some(terminal_def.address.into()),
            shell_title: terminal_def.title.shell_title,
            override_title: terminal_def.title.override_title.map(|s| MaybeString { s }),
            order: terminal_def.order,
        }
    }
}

impl TerminalDefProto {
    pub fn client_address(&self) -> &[String] {
        fn aux(proto: &TerminalDefProto) -> Option<&[String]> {
            let address = proto.address.as_ref()?;
            Some(address.client_address())
        }
        aux(self).unwrap_or(&[])
    }
}

impl From<TerminalAddressProto> for TerminalAddress {
    fn from(proto: TerminalAddressProto) -> Self {
        Self {
            id: proto.terminal_id.into(),
            via: ClientAddress::from(proto.via.unwrap_or_default()),
        }
    }
}

impl From<TerminalAddress> for TerminalAddressProto {
    fn from(address: TerminalAddress) -> Self {
        Self {
            terminal_id: address.id.to_string(),
            via: (!address.via.is_empty()).then(|| ClientAddressProto {
                via: address.via.iter().map(ClientName::to_string).collect(),
            }),
        }
    }
}

impl TerminalAddressProto {
    pub fn client_address(&self) -> &[String] {
        fn aux(proto: &TerminalAddressProto) -> Option<&[String]> {
            let via = proto.via.as_ref()?;
            Some(via.via.as_slice())
        }
        aux(self).unwrap_or(&[])
    }
}

impl From<RegisterTerminalRequest> for RegisterTerminalRequestProto {
    fn from(request: RegisterTerminalRequest) -> Self {
        let mut proto = Self {
            mode: Default::default(),
            def: Some(request.def.into()),
        };
        proto.set_mode(request.mode.into());
        return proto;
    }
}

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

impl TryFrom<RegisterTerminalModeProto> for RegisterTerminalMode {
    type Error = Status;

    fn try_from(proto: RegisterTerminalModeProto) -> Result<Self, Self::Error> {
        Ok(match proto {
            RegisterTerminalModeProto::Unspecified => {
                return Err(Status::invalid_argument("mode"));
            }
            RegisterTerminalModeProto::Create => Self::Create,
            RegisterTerminalModeProto::Reopen => Self::Reopen,
        })
    }
}

impl From<RegisterTerminalMode> for RegisterTerminalModeProto {
    fn from(mode: RegisterTerminalMode) -> Self {
        match mode {
            RegisterTerminalMode::Create => Self::Create,
            RegisterTerminalMode::Reopen => Self::Reopen,
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
