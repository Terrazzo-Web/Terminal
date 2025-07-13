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
        #[server(protocol = ::server_fn::Http<::server_fn::codec::Json, ::server_fn::codec::Json>)]
        #[cfg_attr(feature = "server", nameth::nameth)]
        pub async fn get(remote: Option<ClientAddress>) -> Result<ty::Type, ServerFnError> {
            Ok(remote::GET_REMOTE_FN
                .call(remote.unwrap_or_default(), remote::GetRequest {})
                .await?)
        }

        #[cfg(feature = "client")]
        use crate::frontend::remotes::Remote;

        #[cfg(feature = "client")]
        pub async fn set(
            remote: Remote,
            value: ty::Type,
        ) -> Result<(), ServerFnError> {
            use std::pin::Pin;
            use std::sync::OnceLock;
            use std::time::Duration;
            use terrazzo::prelude::diagnostics::warn;
            use terrazzo::widgets::debounce::DoDebounce as _;

            struct ThreadSafe(
                Box<dyn Fn((Remote, ty::Type)) -> Pin<Box<dyn Future<Output = ()>>>>,
            );

            unsafe impl Send for ThreadSafe {}
            unsafe impl Sync for ThreadSafe {}

            static DEBOUNCED_SET: OnceLock<ThreadSafe> = OnceLock::new();
            const STORE_STATE_DEBOUNCE_DELAY: Duration = Duration::from_millis(100);

            let debounced_set = DEBOUNCED_SET.get_or_init(|| {
                ThreadSafe(Box::new(
                    STORE_STATE_DEBOUNCE_DELAY.async_debounce(|(remote, value)| async move {
                        set_impl(remote, value)
                            .await
                            .unwrap_or_else(|error| warn!("Failed to save: {error}"))
                    }),
                ))
            });
            let debounced_set = &*debounced_set.0;

            let () = debounced_set((remote, value)).await;
            Ok(())
        }

        #[cfg_attr(feature = "server", allow(unused))]
        #[server(protocol = ::server_fn::Http<::server_fn::codec::Json, ::server_fn::codec::Json>)]
        #[cfg_attr(feature = "server", nameth::nameth)]
        async fn set_impl(
            remote: Option<ClientAddress>,
            value: ty::Type,
        ) -> Result<(), ServerFnError> {
            Ok(remote::SET_REMOTE_FN
                .call(remote.unwrap_or_default(), remote::SetRequest { value })
                .await?)
        }

        #[cfg(feature = "server")]
        mod remote {
            use std::future::ready;

            use const_format::formatcp;
            use serde::Deserialize;
            use serde::Serialize;

            use crate::backend::client_service::remote_fn_service;

            #[derive(Debug, Default, Serialize, Deserialize)]
            #[serde(default)]
            pub struct GetRequest {}

            #[derive(Debug, Default, Serialize, Deserialize)]
            #[serde(default)]
            pub struct SetRequest {
                #[cfg_attr(not(feature = "diagnostics"), serde(rename = "v"))]
                pub value: super::ty::Type,
            }

            remote_fn_service::declare_remote_fn!(
                GET_REMOTE_FN,
                formatcp!("{}-state-{}", super::GET, stringify!($name)),
                GetRequest,
                super::ty::Type,
                |_server, _: GetRequest| {
                    let state = super::STATE.lock().expect(stringify!($name));
                    ready(Ok::<super::ty::Type, StateError>(
                        state.as_ref().cloned().unwrap_or_default(),
                    ))
                }
            );

            remote_fn_service::declare_remote_fn!(
                SET_REMOTE_FN,
                formatcp!("{}-state-{}", super::SET_IMPL, stringify!($name)),
                SetRequest,
                (),
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
