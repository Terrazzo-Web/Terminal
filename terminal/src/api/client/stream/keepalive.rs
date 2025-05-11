use std::sync::Arc;
use std::time::Duration;

use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use tracing::Instrument as _;
use tracing::debug;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use crate::api::client::request::BASE_URL;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_correlation_id;
use crate::api::client::request::set_headers;
use crate::api::client::stream::pipe::PIPE;
use crate::frontend::utils::sleep;

#[nameth]
pub fn keepalive(
    keepalive_ttl: Duration,
    correlation_id: Arc<str>,
    mut end_of_pipe: oneshot::Receiver<()>,
) {
    let task = async move {
        loop {
            let sleep = Box::pin(sleep(keepalive_ttl));
            let next = futures::future::select(sleep, end_of_pipe);
            match next.await {
                futures::future::Either::Left((sleep, end_of_pipe2)) => {
                    if let Err(error) = sleep {
                        warn!("Failed to wait for next keep-alive ping: {error}");
                        return;
                    }
                    end_of_pipe = end_of_pipe2;
                }
                futures::future::Either::Right((end_of_pipe, _sleep)) => {
                    if let Err(oneshot::Canceled) = end_of_pipe {
                        warn!("The pipe was dropped");
                    }
                    break;
                }
            }
            match send_keepalive(&correlation_id).await {
                Ok(()) => {}
                Err(error) => {
                    warn!("Keep-alive failed: {error}");
                    return;
                }
            };
        }
        debug!("Keep-alive finished");
    };
    spawn_local(task.in_current_span());
}

async fn send_keepalive(correlation_id: &str) -> Result<(), KeepaliveError> {
    debug!("Send keep-alive");
    let response = send_request(
        Method::POST,
        format!("{BASE_URL}/stream/{PIPE}/{KEEPALIVE}"),
        set_headers(set_correlation_id(correlation_id)),
    )
    .await?;
    debug! { "Keep-alive returned {} {}", response.status(), response.status_text() };
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum KeepaliveError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}
