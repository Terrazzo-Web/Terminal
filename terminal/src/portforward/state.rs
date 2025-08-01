use std::sync::Arc;

use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use crate::api::client_address::ClientAddress;
use crate::portforward::schema::PortForward;

#[server(protocol = Http<Json, Json>)]
#[cfg_attr(feature = "server", nameth::nameth)]
pub async fn store_port_forwards(
    remote: Option<ClientAddress>,
    port_forwards: Arc<[PortForward]>,
) -> Result<(), ServerFnError> {
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    Ok(backend::STORE_PORT_FORWARDS_FN
        .call(remote.unwrap_or_default(), port_forwards)
        .await?)
}

#[server(protocol = Http<Json, Json>)]
#[cfg_attr(feature = "server", nameth::nameth)]
pub async fn load_port_forwards(
    remote: Option<ClientAddress>,
) -> Result<Arc<[PortForward]>, ServerFnError> {
    Ok(backend::LOAD_PORT_FORWARDS_FN
        .call(remote.unwrap_or_default(), ())
        .await?)
}

#[cfg(feature = "server")]
mod backend {
    use std::future::ready;
    use std::sync::Arc;
    use std::sync::Mutex;

    use crate::backend::client_service::port_forward_service::bind::BindError;
    use crate::backend::client_service::remote_fn_service;
    use crate::portforward::schema::PortForward;

    static STATE: Mutex<Option<Arc<[PortForward]>>> = Mutex::new(None);

    remote_fn_service::declare_remote_fn!(
        STORE_PORT_FORWARDS_FN,
        super::STORE_PORT_FORWARDS,
        Arc<[PortForward]>,
        (),
        |server, port_forwards| {
            let server = server.clone();
            async move {
                let old = {
                    let mut old = STATE.lock().expect(super::STORE_PORT_FORWARDS);
                    *old = Some(port_forwards.clone());
                    old.clone()
                };

                use super::super::engine;
                let old = old.as_deref().unwrap_or_default();
                engine::process(&server, old, &port_forwards).await?;
                Ok::<(), BindError>(())
            }
        }
    );

    remote_fn_service::declare_remote_fn!(
        LOAD_PORT_FORWARDS_FN,
        super::LOAD_PORT_FORWARDS,
        (),
        Arc<[PortForward]>,
        |_server, ()| {
            let state = STATE.lock().expect(super::LOAD_PORT_FORWARDS).clone();
            ready(Ok::<_, tonic::Status>(state.unwrap_or_default()))
        }
    );
}
