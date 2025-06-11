macro_rules! make_state {
    ($name:ident, $ty:ty) => {
        pub mod $name {
            use server_fn::ServerFnError;
            use terrazzo::server;

            pub mod ty {
                pub type Type = $ty;

                #[allow(unused)]
                pub use super::super::*;
            }

            use crate::api::client_address::ClientAddress;

            #[cfg(feature = "server")]
            static STATE: std::sync::Mutex<Option<ty::Type>> = std::sync::Mutex::new(None);

            #[cfg_attr(feature = "server", allow(unused))]
            #[server]
            #[cfg_attr(feature = "server", nameth::nameth)]
            pub async fn get(address: Option<ClientAddress>) -> Result<ty::Type, ServerFnError> {
                Ok(remote::GET_REMOTE_FN
                    .call(address.unwrap_or_default(), remote::GetRequest {})
                    .await?)
            }

            #[cfg_attr(feature = "server", allow(unused))]
            #[server]
            #[cfg_attr(feature = "server", nameth::nameth)]
            pub async fn set(
                address: Option<ClientAddress>,
                value: ty::Type,
            ) -> Result<(), ServerFnError> {
                Ok(remote::SET_REMOTE_FN
                    .call(address.unwrap_or_default(), remote::SetRequest { value })
                    .await?)
            }

            #[cfg(feature = "server")]
            mod remote {
                use std::future::ready;

                use const_format::formatcp;
                use serde::Deserialize;
                use serde::Serialize;

                use crate::backend::client_service::remote_fn;

                #[derive(Debug, Default, Serialize, Deserialize)]
                #[serde(default)]
                pub struct GetRequest {}

                #[derive(Debug, Default, Serialize, Deserialize)]
                #[serde(default)]
                pub struct SetRequest {
                    pub value: super::ty::Type,
                }

                remote_fn::declare_remote_fn!(
                    GET_REMOTE_FN,
                    formatcp!("{}-state-{}", super::GET, stringify!($name)),
                    |_server, _: GetRequest| {
                        let state = super::STATE.lock().expect(stringify!($name));
                        ready(Ok::<super::ty::Type, StateError>(
                            state.as_ref().cloned().unwrap_or_default(),
                        ))
                    }
                );

                remote_fn::declare_remote_fn!(
                    SET_REMOTE_FN,
                    formatcp!("{}-state-{}", super::SET, stringify!($name)),
                    |_server, arg: SetRequest| {
                        let mut state = super::STATE.lock().expect(stringify!($name));
                        *state = Some(arg.value);
                        ready(Ok::<(), StateError>(()))
                    }
                );

                enum StateError {}

                impl From<StateError> for tonic::Status {
                    fn from(value: StateError) -> Self {
                        match value {}
                    }
                }
            }
        }
    };
}

pub(crate) use make_state;

pub mod app;
