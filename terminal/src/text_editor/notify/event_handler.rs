#![cfg(feature = "server")]

use tokio::sync::mpsc;
use tracing::warn;

use super::*;

pub struct EventHandler {
    pub tx: mpsc::UnboundedSender<Result<NotifyResponse, ServerFnError>>,
}

impl notify::EventHandler for EventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
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
                match self.tx.send(Err(error.into())) {
                    Ok(()) => {}
                    Err(error) => warn!("Watcher failed {error}"),
                };
                return;
            }
        };
        for path in paths {
            let response = NotifyResponse {
                path: path.to_string_lossy().to_string(),
                kind,
            };
            match self.tx.send(Ok(response)) {
                Ok(()) => {}
                Err(error) => warn!("Watcher failed {error}"),
            }
        }
    }
}
