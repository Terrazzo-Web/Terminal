macro_rules! make_state {
    ($name:ident, $ty:ty) => {
        pub mod $name {
            use server_fn::ServerFnError;
            use terrazzo::server;

            #[allow(unused)]
            use super::*;

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
            pub async fn set(value: $ty) -> Result<(), ServerFnError> {
                let mut state = STATE.lock().expect(stringify!($name));
                *state = Some(value);
                Ok(())
            }
        }
    };
}

pub(crate) use make_state;

pub mod app;
