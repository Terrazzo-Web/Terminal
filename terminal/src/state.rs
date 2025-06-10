macro_rules! make_state {
    ($name:ident, $ty:ty) => {
        pub mod $name {
            use nameth::nameth;
            use server_fn::ServerFnError;
            use terrazzo::server;

            #[allow(unused)]
            use super::*;
            use crate::api::client_address::ClientAddress;

            #[cfg(feature = "server")]
            static STATE: std::sync::Mutex<Option<$ty>> = std::sync::Mutex::new(None);

            #[cfg_attr(feature = "server", allow(unused))]
            #[server]
            pub async fn get() -> Result<$ty, ServerFnError> {
                let state = STATE.lock().expect(stringify!($name));
                Ok(state.as_ref().cloned().unwrap_or_default())
            }

            #[cfg_attr(feature = "server", allow(unused))]
            #[server]
            #[nameth]
            pub async fn set(
                address: Option<ClientAddress>,
                value: $ty,
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
                use trz_gateway_server::server::Server;

                #[allow(unused)]
                use super::*;
                use crate::backend::client_service::remote_fn;
                use crate::backend::client_service::remote_fn::RemoteFn;
                use crate::backend::client_service::remote_fn::RemoteFnResult;

                pub static SET_REMOTE_FN: RemoteFn = RemoteFn {
                    name: formatcp!("{}-state-{}", super::SET, stringify!($name)),
                    callback: set,
                };

                inventory::submit! { SET_REMOTE_FN }

                #[derive(Debug, Serialize, Deserialize)]
                pub struct SetRequest {
                    pub value: $ty,
                }

                fn set(server: &Server, arg: &str) -> RemoteFnResult {
                    let load_file = remote_fn::uplift(|_server, arg: SetRequest| {
                        let mut state = super::STATE.lock().expect(stringify!($name));
                        *state = Some(arg.value);
                        ready(Ok::<_, StateError>(()))
                    });
                    Box::pin(load_file(server, arg))
                }

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
