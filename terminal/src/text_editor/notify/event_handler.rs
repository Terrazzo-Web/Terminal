#![cfg(feature = "server")]

use server_fn::ServerFnError;
use tokio::sync::mpsc;
use tracing::warn;

use super::server_fn::EventKind;
use super::server_fn::FileEventKind;
use super::server_fn::NotifyResponse;
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
                    notify::EventKind::Create { .. } => FileEventKind::Create,
                    notify::EventKind::Modify { .. } => FileEventKind::Modify,
                    notify::EventKind::Remove { .. } => FileEventKind::Delete,
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
                kind: EventKind::File(kind),
            };
            match tx.send(Ok(response)) {
                Ok(()) => {}
                Err(error) => warn!("Watcher failed {error}"),
            }
        }
    }
}
