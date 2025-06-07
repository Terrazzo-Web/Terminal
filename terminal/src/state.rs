macro_rules! make_state {
    ($name:ident, $ty:ty) => {
        pub mod $name {
            use server_fn::ServerFnError;
            use terrazzo::server;

            #[cfg(feature = "server")]
            static STATE: std::sync::Mutex<Option<$ty>> = std::sync::Mutex::new(None);

            #[allow(unused)]
            #[server]
            pub async fn get() -> Result<$ty, ServerFnError> {
                Ok(STATE
                    .lock()
                    .expect(stringify!($name))
                    .as_ref()
                    .cloned()
                    .unwrap_or_default())
            }

            #[allow(unused)]
            #[server]
            pub async fn set(value: $ty) -> Result<(), ServerFnError> {
                *STATE.lock().expect(stringify!($name)) = Some(value);
                Ok(())
            }
        }
    };
}

pub(crate) use make_state;
