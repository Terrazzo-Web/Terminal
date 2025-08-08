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
    port_forwards: Arc<Vec<PortForward>>,
) -> Result<(), ServerFnError> {
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(backend::STORE_PORT_FORWARDS_FN
        .call(remote.unwrap_or_default(), port_forwards)
        .await?)
}

#[server(protocol = Http<Json, Json>)]
#[cfg_attr(feature = "server", nameth::nameth)]
pub async fn load_port_forwards(
    remote: Option<ClientAddress>,
) -> Result<Box<Vec<PortForward>>, ServerFnError> {
    Ok(backend::LOAD_PORT_FORWARDS_FN
        .call(remote.unwrap_or_default(), ())
        .await?)
}

#[cfg(feature = "server")]
mod backend {
    use std::future::ready;
    use std::sync::Arc;
    use std::sync::Mutex;

    use crate::backend::client_service::remote_fn_service;
    use crate::portforward::engine::RunningPortForward;
    use crate::portforward::schema::PortForward;

    static STATE: Mutex<Option<Box<[RunningPortForward]>>> = Mutex::new(None);

    remote_fn_service::declare_remote_fn!(
        STORE_PORT_FORWARDS_FN,
        super::STORE_PORT_FORWARDS,
        Arc<Vec<PortForward>>,
        (),
        |server, port_forwards| {
            let server = server.clone();
            async move {
                let (stopping, pending) = {
                    let mut lock = STATE.lock().expect(super::STORE_PORT_FORWARDS);
                    let (running, stopping, pending) =
                        engine::prepare(lock.take().unwrap_or_default(), port_forwards);
                    *lock = Some(running);
                    (stopping, pending)
                };

                for stopping in stopping {
                    let () = stopping.stop().await;
                }

                use super::super::engine;
                let () = engine::process(&server, pending).await;
                Ok::<(), tonic::Status>(())
            }
        }
    );

    remote_fn_service::declare_remote_fn!(
        LOAD_PORT_FORWARDS_FN,
        super::LOAD_PORT_FORWARDS,
        (),
        Box<Vec<PortForward>>,
        |_server, ()| {
            let state = STATE.lock().expect(super::LOAD_PORT_FORWARDS);
            let state = state
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|running| running.port_forward.clone())
                .collect::<Vec<_>>();
            ready(Ok::<_, tonic::Status>(state.into()))
        }
    );
}
