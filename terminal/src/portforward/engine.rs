#![cfg(feature = "server")]

use std::collections::HashMap;
use std::future::ready;
use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::oneshot;
use futures::stream;
use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use super::schema::HostPortDefinition;
use super::schema::PortForward;
use crate::backend::client_service::port_forward_service;
use crate::backend::client_service::port_forward_service::bind::BindError;
use crate::backend::client_service::port_forward_service::bind::BindStream;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptRequest;
use crate::backend::protos::terrazzo::shared::ClientAddress;

pub async fn process(
    server: &Arc<Server>,
    old: &[PortForward],
    new: &[PortForward],
) -> Result<(), BindError> {
    let old = old
        .iter()
        .map(|old| (old.id, old))
        .collect::<HashMap<_, _>>();
    for new in new {
        let () = process_port_forward(server, old.get(&new.id).copied(), new).await?;
    }
    Ok(())
}

async fn process_port_forward(
    server: &Arc<Server>,
    old: Option<&PortForward>,
    new: &PortForward,
) -> Result<(), BindError> {
    if old == Some(new) {
        return Ok(());
    }

    let (eos_tx, eos_rx) = oneshot::channel();
    let eos = stream::once(eos_rx).filter_map(|_| ready(None));
    let requests = stream::once(ready(Ok(PortForwardAcceptRequest {
        remote: new.from.forwarded_remote.as_deref().map(ClientAddress::of),
        host: new.from.host.to_owned(),
        port: new.from.port as i32,
    })))
    .chain(eos);

    let stream = port_forward_service::bind::dispatch(server, requests).await?;
    let span = info_span!("Forward Port", from = %new.from, to = %new.to);
    tokio::spawn(process_bind_stream(new.to.clone(), stream, eos_tx).instrument(span));
    Ok(())
}

async fn process_bind_stream(
    to: HostPortDefinition,
    mut stream: BindStream,
    eos: oneshot::Sender<()>,
) {
    debug!("Start");
    defer!(debug!("End"));

    while let Some(next) = stream.next().await {}

    match eos.send(()) {
        Ok(()) => debug!("Closed PortForward Bind request stream"),
        Err(()) => warn!("Failed to close PortForward Bind request stream"),
    }
}
