use tonic::Status;
use trz_gateway_common::id::ClientName;

use crate::api::client_address::ClientAddress;
use crate::api::shared::terminal_schema::RegisterTerminalMode;
use crate::api::shared::terminal_schema::RegisterTerminalRequest;
use crate::api::shared::terminal_schema::TabTitle;
use crate::api::shared::terminal_schema::TerminalAddress;
use crate::api::shared::terminal_schema::TerminalDef;
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;
use crate::backend::protos::terrazzo::terminal::MaybeString;
use crate::backend::protos::terrazzo::terminal::RegisterTerminalRequest as RegisterTerminalRequestProto;
use crate::backend::protos::terrazzo::terminal::TerminalAddress as TerminalAddressProto;
use crate::backend::protos::terrazzo::terminal::TerminalDef as TerminalDefProto;
use crate::backend::protos::terrazzo::terminal::register_terminal_request::RegisterTerminalMode as RegisterTerminalModeProto;

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
