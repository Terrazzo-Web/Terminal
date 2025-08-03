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
    let requests = stream::once(ready(Ok(PortForwardEndpoint {
        remote: new.from.forwarded_remote.as_deref().map(ClientAddress::of),
        host: new.from.host.to_owned(),
        port: new.from.port as i32,
    })))
    .chain(eos);

    let stream = port_forward_service::bind::dispatch(server, requests).await?;
    let span = info_span!("Forward Port", from = %new.from, to = %new.to);
    tokio::spawn(process_bind_stream(server.clone(), new.clone(), stream, eos_tx).instrument(span));
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

        tokio::spawn(run_stream(server.clone(), port_forward.clone()));
    }
}

async fn run_stream(server: Arc<Server>, port_forward: PortForward) -> Result<(), RunStreamError> {
    let (upload_stream_tx, upload_stream_rx) = oneshot::channel();
    let upload_stream = futures::stream::once(upload_stream_rx)
        .filter_map(|stream| ready(stream.ok()))
        .flatten()
        .map_ok(|response: PortForwardDataResponse| PortForwardDataRequest {
            kind: Some(port_forward_data_request::Kind::Data(response.data)),
        });

    let upload_endpoint = port_forward.to;
    let upload_stream = futures::stream::once(ready(Ok(PortForwardDataRequest {
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
    let download_endpoint = port_forward.from;
    let download_stream = futures::stream::once(ready(Ok(PortForwardDataRequest {
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
