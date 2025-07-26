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

    use crate::backend::client_service::remote_fn_service;
    use crate::portforward::schema::PortForward;

    static STATE: Mutex<Option<Arc<[PortForward]>>> = Mutex::new(None);

    remote_fn_service::declare_remote_fn!(
        STORE_PORT_FORWARDS_FN,
        super::STORE_PORT_FORWARDS,
        Arc<[PortForward]>,
        (),
        |_server, port_forwards| {
            *STATE.lock().expect(super::STORE_PORT_FORWARDS) = Some(port_forwards);
            ready(Ok::<(), tonic::Status>(()))
        }
    );

    remote_fn_service::declare_remote_fn!(
        LOAD_PORT_FORWARDS_FN,
        super::LOAD_PORT_FORWARDS,
        (),
        Arc<[PortForward]>,
        |_server, port_forwards| {
            let state = STATE.lock().expect(super::LOAD_PORT_FORWARDS).clone();
            ready(Ok::<_, tonic::Status>(state))
        }
    );
}
