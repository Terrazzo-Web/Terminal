use tonic::Status;
use trz_gateway_common::id::ClientName;

use crate::api::RegisterTerminalMode;
use crate::api::RegisterTerminalRequest;
use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::api::client_address::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::gateway::client::MaybeString;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalMode as RegisterTerminalModeProto;
use crate::backend::protos::terrazzo::gateway::client::RegisterTerminalRequest as RegisterTerminalRequestProto;
use crate::backend::protos::terrazzo::gateway::client::TerminalAddress as TerminalAddressProto;
use crate::backend::protos::terrazzo::gateway::client::TerminalDef as TerminalDefProto;

impl From<TerminalDefProto> for TerminalDef {
    fn from(proto: TerminalDefProto) -> Self {
        let address = proto.address.unwrap_or_default();
        Self {
            id: address.terminal_id.into(),
            title: TabTitle {
                shell_title: proto.shell_title,
                override_title: proto.override_title.map(|s| s.s),
            },
            order: proto.order,
            via: ClientAddress::from(address.via.unwrap_or_default()),
        }
    }
}

impl From<TerminalDef> for TerminalDefProto {
    fn from(terminal_def: TerminalDef) -> Self {
        Self {
            address: Some(TerminalAddressProto {
                terminal_id: terminal_def.id.to_string(),
                via: (!terminal_def.via.is_empty()).then(|| ClientAddressProto {
                    via: terminal_def
                        .via
                        .iter()
                        .map(|client_name| client_name.to_string())
                        .collect(),
                }),
            }),
            shell_title: terminal_def.title.shell_title,
            override_title: terminal_def.title.override_title.map(|s| MaybeString { s }),
            order: terminal_def.order,
        }
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
