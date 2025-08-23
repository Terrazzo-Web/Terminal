#![cfg(feature = "server")]

use std::collections::HashMap;
use std::collections::HashSet;
use std::future::ready;
use std::sync::Arc;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryFutureExt as _;
use futures::TryStreamExt as _;
use futures::channel::oneshot;
use futures::future::Shared;
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
use self::port_forward_service::bind::IsBindStream;
use self::port_forward_service::download::DownloadLocalError;
use self::port_forward_service::stream::GrpcStreamError;
use self::port_forward_service::upload::UploadLocalError;
use self::protos::PortForwardAcceptResponse;
use self::protos::PortForwardDataRequest;
use self::protos::PortForwardDataResponse;
use self::protos::PortForwardEndpoint;
use self::protos::port_forward_data_request;
use super::schema::PortForward;
use crate::backend::client_service::port_forward_service;
use crate::backend::protos::terrazzo::portforward as protos;
use crate::backend::protos::terrazzo::shared::ClientAddress;
use crate::portforward::engine::retry::BindStreamWithRetry;
use crate::portforward::schema::PortForwardStatus;

mod retry;

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
    ask: Shared<oneshot::Receiver<()>>,
    ack: oneshot::Sender<()>,
}

pub struct PreparedPortForwards {
    pub running: Box<[RunningPortForward]>,
    pub stopping: Box<[RunningPortForward]>,
    pub pending: Box<[PendingPortForward]>,
}

pub fn prepare(old: Box<[RunningPortForward]>, new: Arc<Vec<PortForward>>) -> PreparedPortForwards {
    let mut running = vec![];
    let mut stopping = vec![];
    let mut pending = vec![];
    let mut old = old
        .into_iter()
        .map(|old| (old.port_forward.id, old))
        .collect::<HashMap<_, _>>();
    let new = match Arc::try_unwrap(new) {
        Ok(new) => new,
        Err(new) => new.as_ref().clone(),
    };
    let mut deduplicate = HashSet::new();
    for mut new in new.into_iter() {
        if !deduplicate.insert(new.id) {
            continue;
        }
        let old = old.remove(&new.id);
        if let Some(running_old) = old {
            let old = &running_old.port_forward;
            debug!("Update Port Forward config from {old:?} to {new:?}");
            new.state = old.state.clone();
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

        new.state.lock().status = PortForwardStatus::Pending;
        let (eos_ask_tx, eos_ask_rx) = oneshot::channel();
        let (eos_ack_tx, eos_ack_rx) = oneshot::channel();
        let eos_ask_rx = eos_ask_rx.shared();
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
    PreparedPortForwards {
        running: Box::from(running),
        stopping: Box::from(stopping),
        pending: Box::from(pending),
    }
}

pub async fn process(server: &Arc<Server>, new: Box<[PendingPortForward]>) {
    for new in new {
        let () = process_port_forward(server, new).await;
    }
}

async fn process_port_forward(server: &Arc<Server>, new: PendingPortForward) {
    let PendingPortForward {
        port_forward,
        ask,
        ack,
    } = new;
    if !port_forward.checked {
        port_forward.state.lock().status = PortForwardStatus::Offline;
        return;
    }
    let stream = BindStreamWithRetry::new(server.clone(), port_forward.clone(), ask.clone());

    let span = info_span!("Forward Port", id = port_forward.id, from = %port_forward.from, to = %port_forward.to);
    let process_bind_stream = process_bind_stream(server.clone(), port_forward, stream, ask, ack);
    tokio::spawn(process_bind_stream.instrument(span));
}

async fn get_bind_stream(
    server: Arc<Server>,
    port_forward: PortForward,
    ask: Shared<oneshot::Receiver<()>>,
) -> Result<BindStream, BindError> {
    let requests = stream::once(ready(Ok(PortForwardEndpoint {
        remote: port_forward
            .from
            .forwarded_remote
            .as_deref()
            .map(ClientAddress::of),
        host: port_forward.from.host.to_owned(),
        port: port_forward.from.port as i32,
    })))
    .chain(stream::once(ask.clone()).filter_map(|_| ready(None)));
    let stream = port_forward_service::bind::dispatch(&server, requests).await;
    stream.inspect_err(|error| debug!("Bind failed: {error}"))
}

async fn process_bind_stream(
    server: Arc<Server>,
    port_forward: PortForward,
    mut stream: impl IsBindStream,
    ask_eos: Shared<oneshot::Receiver<()>>,
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

    match &mut port_forward.state.lock().status {
        status @ PortForwardStatus::Pending => *status = PortForwardStatus::Up,
        status @ (PortForwardStatus::Up
        | PortForwardStatus::Offline
        | PortForwardStatus::Failed { .. }) => {
            warn!("Expected status to be pending, got {status:?}")
        }
    };
    while let Some(next) = stream.next().await {
        match next {
            Ok(PortForwardAcceptResponse {}) => (),
            Err(error) => {
                let error = error.message().to_owned();
                warn!("Failed to get the next connection: {error}");
                port_forward.state.lock().status = PortForwardStatus::Failed(error);
                return;
            }
        }

        tokio::spawn(
            run_stream(server.clone(), ask_eos.clone(), port_forward.clone())
                .unwrap_or_else(move |error| warn!("A stream failed with: {error}"))
                .in_current_span(),
        );
    }
}

async fn run_stream(
    server: Arc<Server>,
    ask_eos: Shared<oneshot::Receiver<()>>,
    port_forward: PortForward,
) -> Result<(), RunStreamError> {
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

    let state = port_forward.state;
    {
        let mut lock = state.lock();
        lock.count += 1;
        debug!("Increment count of running streams");
    }
    let decrement = scopeguard::guard((), move |()| {
        state.lock().count -= 1;
        debug!("Decrement count of running streams");
    });
    let decrement = async move {
        drop(decrement);
    };
    let upload_stream = upload_stream
        .take_until(ask_eos)
        .chain(stream::once(Box::pin(decrement)).filter_map(|_| ready(None)));
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

#[cfg(test)]
#[test]
fn duplicate_key() {
    let t: HashMap<i32, &str> = [(1, "a"), (2, "b"), (3, "c")].into_iter().collect();
    assert_eq!(Some(&"a"), t.get(&1));
    let t: HashMap<i32, &str> = [(1, "a"), (2, "b"), (1, "c")].into_iter().collect();
    assert_eq!(Some(&"c"), t.get(&1));
}
