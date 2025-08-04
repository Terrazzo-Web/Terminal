#![cfg(feature = "server")]

use std::collections::HashMap;
use std::future::ready;
use std::sync::Arc;

use futures::StreamExt as _;
use futures::TryStreamExt;
use futures::channel::oneshot;
use futures::stream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tracing::Instrument as _;
use tracing::debug;
use tracing::info_span;
use tracing::warn;
use trz_gateway_server::server::Server;

use self::port_forward_service::bind::BindError;
use self::port_forward_service::bind::BindStream;
use self::port_forward_service::download::DownloadLocalError;
use self::port_forward_service::stream::GrpcStreamError;
use self::port_forward_service::upload::UploadLocalError;
use super::schema::PortForward;
use crate::backend::client_service::port_forward_service;
use crate::backend::protos::terrazzo::portforward::PortForwardAcceptResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;
use crate::backend::protos::terrazzo::portforward::PortForwardDataResponse;
use crate::backend::protos::terrazzo::portforward::PortForwardEndpoint;
use crate::backend::protos::terrazzo::portforward::port_forward_data_request;
use crate::backend::protos::terrazzo::shared::ClientAddress;

pub struct RunningPortForward {
    pub port_forward: PortForward,
    ask: oneshot::Sender<()>,
    ack: oneshot::Receiver<()>,
}

impl RunningPortForward {
    pub async fn stop(self) {
        let Self {
            port_forward,
            ask,
            ack,
        } = self;
        if let Err(()) = ask.send(()) {
            warn!("Failed to stop {port_forward:?}");
        }
        if let Err(error) = ack.await {
            warn!("Failed to stop {port_forward:?}: {error}")
        }
    }
}

pub struct PendingPortForward {
    port_forward: PortForward,
    ask: oneshot::Receiver<()>,
    ack: oneshot::Sender<()>,
}

pub fn prepare(
    old: Box<[RunningPortForward]>,
    new: Arc<Vec<PortForward>>,
) -> (
    Box<[RunningPortForward]>,
    Box<[RunningPortForward]>,
    Box<[PendingPortForward]>,
) {
    let mut running = vec![];
    let mut stopping = vec![];
    let mut pending = vec![];
    let mut old = old
        .into_iter()
        .map(|old| (old.port_forward.id, old))
        .collect::<HashMap<_, _>>();
    let new = match Arc::try_unwrap(new) {
        Ok(new) => Box::from(new),
        Err(new) => Box::new(new.as_ref().clone()),
    };
    for new in new.into_iter() {
        let old = old.remove(&new.id);
        if let Some(running_old) = old {
            let old = &running_old.port_forward;
            debug!("Update Port Forward config from {old:?} to {new:?}");
            if old == &new {
                debug!("Port forward config did not change: {old:?}");
                running.push(running_old);
                continue;
            } else {
                stopping.push(running_old);
            }
        } else {
            debug!("Add Port Forward config {new:?}");
        }

        let (eos_ask_tx, eos_ask_rx) = oneshot::channel();
        let (eos_ack_tx, eos_ack_rx) = oneshot::channel();
        running.push(RunningPortForward {
            port_forward: new.clone(),
            ask: eos_ask_tx,
            ack: eos_ack_rx,
        });
        pending.push(PendingPortForward {
            port_forward: new,
            ask: eos_ask_rx,
            ack: eos_ack_tx,
        });
    }
    (Box::from(running), Box::from(stopping), Box::from(pending))
}

pub async fn process(
    server: &Arc<Server>,
    new: Box<[PendingPortForward]>,
) -> Result<(), BindError> {
    for new in new {
        let () = process_port_forward(server, new).await?;
    }
    Ok(())
}

async fn process_port_forward(
    server: &Arc<Server>,
    new: PendingPortForward,
) -> Result<(), BindError> {
    let PendingPortForward {
        port_forward,
        ask,
        ack,
    } = new;
    let requests = stream::once(ready(Ok(PortForwardEndpoint {
        remote: port_forward
            .from
            .forwarded_remote
            .as_deref()
            .map(ClientAddress::of),
        host: port_forward.from.host.to_owned(),
        port: port_forward.from.port as i32,
    })))
    .chain(stream::once(ask).filter_map(|_| ready(None)));

    let stream = port_forward_service::bind::dispatch(server, requests)
        .await
        .inspect_err(|error| debug!("Bind failed: {error}"))?;
    let span = info_span!("Forward Port", from = %port_forward.from, to = %port_forward.to);
    tokio::spawn(process_bind_stream(server.clone(), port_forward, stream, ack).instrument(span));
    Ok(())
}

async fn process_bind_stream(
    server: Arc<Server>,
    port_forward: PortForward,
    mut stream: BindStream,
    eos: oneshot::Sender<()>,
) {
    debug!("Start");
    defer!(debug!("End"));

    defer! {
        match eos.send(()) {
            Ok(()) => debug!("Closed PortForward Bind request stream"),
            Err(()) => warn!("Failed to close PortForward Bind request stream"),
        }
    }

    while let Some(next) = stream.next().await {
        match next {
            Ok(PortForwardAcceptResponse {}) => (),
            Err(error) => {
                warn!("Failed to get the next connection: {error}");
                return;
            }
        }

        tokio::spawn(run_stream(server.clone(), port_forward.clone()).in_current_span());
    }
}

async fn run_stream(server: Arc<Server>, port_forward: PortForward) -> Result<(), RunStreamError> {
    let (upload_stream_tx, upload_stream_rx) = oneshot::channel();
    let upload_stream = stream::once(upload_stream_rx)
        .filter_map(|stream| ready(stream.ok()))
        .flatten()
        .map_ok(|response: PortForwardDataResponse| PortForwardDataRequest {
            kind: Some(port_forward_data_request::Kind::Data(response.data)),
        });

    let upload_endpoint = port_forward.from;
    let upload_stream = stream::once(ready(Ok(PortForwardDataRequest {
        kind: Some(port_forward_data_request::Kind::Endpoint(
            PortForwardEndpoint {
                remote: upload_endpoint
                    .forwarded_remote
                    .as_deref()
                    .map(ClientAddress::of),
                host: upload_endpoint.host.clone(),
                port: upload_endpoint.port as i32,
            },
        )),
    })))
    .chain(upload_stream);

    let download_stream = port_forward_service::download::download(&server, upload_stream)
        .await?
        .map_ok(|response: PortForwardDataResponse| PortForwardDataRequest {
            kind: Some(port_forward_data_request::Kind::Data(response.data)),
        });
    let download_endpoint = port_forward.to;
    let download_stream = stream::once(ready(Ok(PortForwardDataRequest {
        kind: Some(port_forward_data_request::Kind::Endpoint(
            PortForwardEndpoint {
                remote: download_endpoint
                    .forwarded_remote
                    .as_deref()
                    .map(ClientAddress::of),
                host: download_endpoint.host.clone(),
                port: download_endpoint.port as i32,
            },
        )),
    })))
    .chain(download_stream);

    let upload_stream = port_forward_service::upload::upload(&server, download_stream).await?;
    let () = upload_stream_tx
        .send(upload_stream)
        .map_err(|_upload_stream| RunStreamError::SetUploadStream)?;
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RunStreamError {
    #[error("[{n}] {0}", n = self.name())]
    UploadStream(#[from] GrpcStreamError<UploadLocalError>),

    #[error("[{n}] {0}", n = self.name())]
    DownloadStream(#[from] GrpcStreamError<DownloadLocalError>),

    #[error("[{n}] Failed to stich the upload stream", n = self.name())]
    SetUploadStream,
}
