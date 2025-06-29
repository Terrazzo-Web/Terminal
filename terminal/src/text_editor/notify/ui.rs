#![cfg(feature = "client")]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::Weak;

use futures::SinkExt;
use futures::StreamExt as _;
use futures::channel::mpsc;
use terrazzo::autoclone;
use terrazzo::prelude::diagnostics::*;
use wasm_bindgen_futures::spawn_local;

use crate::text_editor::notify::*;

pub struct NotifyService {
    request: mpsc::UnboundedSender<Result<NotifyRequest, ServerFnError>>,
    handlers: Handlers,
}

type Handlers = Arc<Mutex<HashMap<usize, Box<dyn Fn(&NotifyResponse)>>>>;

#[derive(Clone)]
pub struct NotifyRegistration {
    id: usize,
    notify_service: Weak<NotifyService>,
}

impl NotifyService {
    #[autoclone]
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::unbounded();
        let handlers = Handlers::default();
        spawn_local(async move {
            autoclone!(handlers);
            let Ok(mut response) = super::notify(request_rx.into())
                .await
                .inspect_err(|error| warn!("Notify stream failed: {error}"))
            else {
                return;
            };
            while let Some(response) = response.next().await {
                match response {
                    Ok(response) => {
                        debug!("{response:?}");
                        for handler in handlers.lock().unwrap().values() {
                            handler(&response);
                        }
                    }
                    Err(error) => {
                        warn!("{error:?}");
                    }
                }
            }
        });
        Self {
            request: request_tx,
            handlers,
        }
    }

    pub fn watch(&self, base_path: &str, file_path: &str) {
        let path = Path::new(base_path).join(file_path);
        let path = path.to_string_lossy().as_ref().to_owned().into();
        self.send(Ok(NotifyRequest::Watch { path }));
    }

    pub fn unwatch(&self, base_path: &str, file_path: &str) {
        let path = Path::new(base_path).join(file_path);
        let path = path.to_string_lossy().as_ref().to_owned().into();
        self.send(Ok(NotifyRequest::UnWatch { path }));
    }

    fn send(&self, notify_request: Result<NotifyRequest, ServerFnError>) {
        let mut request = self.request.clone();
        spawn_local(async move {
            let () = request
                .send(notify_request)
                .await
                .unwrap_or_else(|error| warn!("Failed to send notify request: {error}"));
        });
    }

    pub fn add_handler(
        self: &Arc<Self>,
        handler: impl Fn(&NotifyResponse) + 'static,
    ) -> NotifyRegistration {
        let registration = NotifyRegistration::new(self);
        self.handlers
            .lock()
            .unwrap()
            .insert(registration.id, Box::new(handler));
        registration
    }
}

impl NotifyRegistration {
    fn new(notify_service: &Arc<NotifyService>) -> Self {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT.fetch_add(1, SeqCst),
            notify_service: Arc::downgrade(notify_service),
        }
    }
}

impl Drop for NotifyRegistration {
    fn drop(&mut self) {
        let Some(notify_service) = self.notify_service.upgrade() else {
            return;
        };
        notify_service.handlers.lock().unwrap().remove(&self.id);
    }
}
