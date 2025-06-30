#![cfg(feature = "client")]

use std::collections::HashMap;
use std::future::ready;
use std::path::Path;
use std::sync::Mutex;
use std::sync::Weak;

use futures::SinkExt;
use futures::StreamExt as _;
use futures::channel::mpsc;
use scopeguard::defer;
use terrazzo::autoclone;
use terrazzo::prelude::diagnostics::debug;
use terrazzo::prelude::diagnostics::debug_span;
use terrazzo::prelude::diagnostics::trace;
use terrazzo::prelude::diagnostics::warn;
use wasm_bindgen_futures::spawn_local;

use crate::frontend::remotes::Remote;
use crate::text_editor::notify::*;

pub struct NotifyService {
    remote: Remote,
    inner: Arc<Mutex<Option<NotifyServiceImpl>>>,
}

struct NotifyServiceImpl {
    request: mpsc::UnboundedSender<Result<NotifyRequest, ServerFnError>>,
    handlers: Handlers,
}

type Handlers = Arc<Mutex<HashMap<usize, Arc<dyn Fn(&NotifyResponse)>>>>;

pub struct NotifyRegistration {
    id: usize,
    notify_service: Weak<NotifyService>,
}

impl NotifyService {
    pub fn new(remote: Remote) -> Self {
        Self {
            remote: remote.clone(),
            inner: Arc::new(Mutex::new(None)),
        }
    }

    fn inner<R>(&self, f: impl FnOnce(&mut NotifyServiceImpl) -> R) -> R {
        let mut inner = self.inner.lock().unwrap();
        let inner = &mut *inner;
        let inner =
            inner.get_or_insert_with(|| NotifyServiceImpl::new(self.remote.clone(), &self.inner));
        f(inner)
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
        let mut request = self.inner(|inner| inner.request.clone());
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
    ) -> Arc<NotifyRegistration> {
        let registration = NotifyRegistration::new(self);
        let handlers = self.inner(|inner| inner.handlers.clone());
        handlers
            .lock()
            .unwrap()
            .insert(registration.id, Arc::new(handler));
        Arc::new(registration)
    }
}

impl NotifyServiceImpl {
    #[autoclone]
    fn new(remote: Remote, inner: &Arc<Mutex<Option<NotifyServiceImpl>>>) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded();
        let handlers = Handlers::default();
        let request = futures::stream::once(ready(Ok(NotifyRequest::Start {
            remote: remote.unwrap_or_default(),
        })))
        .chain(request_rx);
        spawn_local(async move {
            autoclone!(inner, handlers);
            let Ok(mut response) = super::notify(request.into())
                .await
                .inspect_err(|error| warn!("Notify stream failed: {error}"))
            else {
                return;
            };
            while let Some(response) = response.next().await {
                match response {
                    Ok(response) => {
                        debug!("{response:?}");
                        let handlers = {
                            let lock = handlers.lock().unwrap();
                            lock.values().cloned().collect::<Vec<_>>()
                        };
                        for handler in handlers {
                            handler(&response);
                        }
                    }
                    Err(error) => {
                        warn!("{error:?}");
                        inner.lock().unwrap().take();
                        return;
                    }
                }
            }
        });
        Self {
            request: request_tx,
            handlers,
        }
    }
}

impl NotifyRegistration {
    fn new(notify_service: &Arc<NotifyService>) -> Self {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, SeqCst);
        debug!("Create notify registration {id}");
        Self {
            id,
            notify_service: Arc::downgrade(notify_service),
        }
    }
}

impl Drop for NotifyRegistration {
    fn drop(&mut self) {
        let _span = debug_span!("Drop notify registration", id = self.id).entered();
        debug!("Start");
        defer!(debug!("End"));
        let Some(notify_service) = self.notify_service.upgrade() else {
            trace!("Notify service is dropped");
            return;
        };
        trace!("Getting handlers");
        let handlers = notify_service.inner(|inner| inner.handlers.clone());
        trace!("Acquire lock");
        let mut handlers = handlers.lock().unwrap();
        trace!("Removing registration");
        handlers.remove(&self.id);
    }
}
