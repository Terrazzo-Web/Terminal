#![cfg(feature = "server")]

use tokio::sync::mpsc;
use tracing::warn;

use super::EventKind;
use super::NotifyResponse;
use super::ServerFnError;
use crate::utils::more_path::MorePath as _;

pub fn make_event_handler(
    tx: mpsc::UnboundedSender<Result<NotifyResponse, ServerFnError>>,
) -> impl notify::EventHandler {
    move |event: Result<notify::Event, notify::Error>| {
        let (kind, paths) = match event {
            Ok(event) => {
                let kind = match event.kind {
                    notify::EventKind::Any
                    | notify::EventKind::Access { .. }
                    | notify::EventKind::Other => return,
                    notify::EventKind::Create { .. } => EventKind::Create,
                    notify::EventKind::Modify { .. } => EventKind::Modify,
                    notify::EventKind::Remove { .. } => EventKind::Delete,
                };
                (kind, event.paths)
            }
            Err(error) => {
                match tx.send(Err(error.into())) {
                    Ok(()) => {}
                    Err(error) => warn!("Watcher failed {error}"),
                };
                return;
            }
        };
        for path in paths {
            let response = NotifyResponse {
                path: path.to_owned_string(),
                kind,
            };
            match tx.send(Ok(response)) {
                Ok(()) => {}
                Err(error) => warn!("Watcher failed {error}"),
            }
        }
    }
}
