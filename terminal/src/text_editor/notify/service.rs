#![cfg(feature = "server")]

use std::path::Path;

use futures::FutureExt;
use futures::StreamExt as _;
use futures::TryFutureExt;
use futures::channel::oneshot;
use futures::stream::PollNext;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use notify::RecommendedWatcher;
use notify::Watcher;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::NotifyRequest;
use super::NotifyResponse;
use super::event_handler;

pub fn notify(
    request: BoxedStream<NotifyRequest, ServerFnError>,
) -> Result<BoxedStream<NotifyResponse, ServerFnError>, ServerFnError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let (eos_tx, eos_rx) = oneshot::channel();
    let eos_rx = eos_rx.shared();
    tokio::spawn(async move {
        let mut request = request;
        let mut watcher = None;
        while let Some(request) = request.next().await {
            match process_request(request, &mut watcher, &tx) {
                Ok(()) => {}
                Err(error) => {
                    let _ = eos_tx.send(Err(error.into()));
                    return;
                }
            }
        }
    });
    let rx = UnboundedReceiverStream::new(rx);
    let rx = futures::stream::select_with_strategy(
        rx.take_until(eos_rx.clone()),
        futures::stream::once(eos_rx.unwrap_or_else(|e| Err(e.into()))),
        |&mut ()| PollNext::Left,
    );
    Ok(rx.into())
}

fn process_request(
    request: Result<NotifyRequest, ServerFnError>,
    watcher: &mut Option<RecommendedWatcher>,
    tx: &mpsc::UnboundedSender<Result<NotifyResponse, ServerFnError>>,
) -> Result<(), NotifyError> {
    match request.map_err(NotifyError::BadRequest)? {
        NotifyRequest::Start { remote: _ } => {
            *watcher = Some(
                notify::recommended_watcher(event_handler::EventHandler { tx: tx.clone() })
                    .map_err(NotifyError::CreateWatcher)?,
            );
        }
        NotifyRequest::Watch { full_path } => watcher
            .as_mut()
            .ok_or(NotifyError::WatcherNotSet)?
            .watch(
                Path::new(full_path.as_ref()),
                notify::RecursiveMode::NonRecursive,
            )
            .map_err(NotifyError::Watch)?,
        NotifyRequest::UnWatch { full_path } => watcher
            .as_mut()
            .ok_or(NotifyError::WatcherNotSet)?
            .unwatch(Path::new(full_path.as_ref()))
            .map_err(NotifyError::Unwatch)?,
    }
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum NotifyError {
    #[error("[{n}] {0}", n = self.name())]
    CreateWatcher(notify::Error),

    #[error("[{n}] {0}", n = self.name())]
    Watch(notify::Error),

    #[error("[{n}] {0}", n = self.name())]
    Unwatch(notify::Error),

    #[error("[{n}] Watcher not set", n = self.name())]
    WatcherNotSet,

    #[error("[{n}] {0}", n = self.name())]
    BadRequest(ServerFnError),
}
