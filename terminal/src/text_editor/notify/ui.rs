#![cfg(feature = "client")]

use std::collections::HashMap;
use std::collections::hash_map;
use std::future::ready;
use std::path::Path;
use std::sync::Mutex;

use futures::SinkExt;
use futures::StreamExt as _;
use futures::channel::mpsc;
use scopeguard::defer;
use terrazzo::autoclone;
use terrazzo::prelude::Ptr;
use terrazzo::prelude::diagnostics::Instrument as _;
use terrazzo::prelude::diagnostics::debug;
use terrazzo::prelude::diagnostics::debug_span;
use terrazzo::prelude::diagnostics::trace;
use terrazzo::prelude::diagnostics::warn;
use wasm_bindgen_futures::spawn_local;

use crate::frontend::remotes::Remote;
use crate::text_editor::file_path::FilePath;
use crate::text_editor::notify::Arc;
use crate::text_editor::notify::NotifyRequest;
use crate::text_editor::notify::NotifyResponse;
use crate::text_editor::notify::ServerFnError;

pub(in crate::text_editor) struct NotifyService {
    remote: Remote,
    inner: Ptr<Mutex<Option<NotifyServiceImpl>>>,
}

struct NotifyServiceImpl {
    request: mpsc::UnboundedSender<Result<NotifyRequest, ServerFnError>>,
    handlers: Handlers,
}

type Handlers =
    Arc<Mutex<HashMap<FilePath<Arc<str>>, HashMap<usize, std::rc::Weak<NotifyRegistration>>>>>;

#[must_use]
pub struct NotifyRegistration {
    id: usize,
    full_path: FilePath<Arc<str>>,
    notify_service: std::rc::Weak<NotifyService>,
    registration_type: RegistrationType,
    callback: Box<dyn Fn(&NotifyResponse)>,
}

#[derive(Clone, Copy, Debug)]
enum RegistrationType {
    File,
    Folder,
}

impl NotifyService {
    pub fn new(remote: Remote) -> Self {
        Self {
            remote: remote.clone(),
            inner: Ptr::new(Mutex::new(None)),
        }
    }

    fn inner<R>(&self, f: impl FnOnce(&mut NotifyServiceImpl) -> R) -> R {
        let mut inner = self.inner.lock().unwrap();
        let inner = &mut *inner;
        let inner =
            inner.get_or_insert_with(|| NotifyServiceImpl::new(self.remote.clone(), &self.inner));
        f(inner)
    }

    #[must_use]
    pub fn watch_file(
        self: &Ptr<Self>,
        full_path: &FilePath<Arc<str>>,
        callback: impl Fn(&NotifyResponse) + 'static,
    ) -> Ptr<NotifyRegistration> {
        self.add_handler(full_path, RegistrationType::File, callback)
    }

    #[must_use]
    pub fn watch_folder(
        self: &Ptr<Self>,
        full_path: &FilePath<Arc<str>>,
        callback: impl Fn(&NotifyResponse) + 'static,
    ) -> Ptr<NotifyRegistration> {
        self.add_handler(full_path, RegistrationType::Folder, callback)
    }

    #[must_use]
    fn add_handler(
        self: &Ptr<Self>,
        full_path: &FilePath<Arc<str>>,
        registration_type: RegistrationType,
        callback: impl Fn(&NotifyResponse) + 'static,
    ) -> Ptr<NotifyRegistration> {
        let _span = debug_span!("Add watch", ?full_path, ?registration_type).entered();
        debug!("Start");
        defer!(debug!("End"));
        let registration =
            NotifyRegistration::new(full_path.clone(), self, registration_type, callback);
        let handlers = self.inner(|inner| inner.handlers.clone());
        let mut handlers = handlers.lock().unwrap();
        let mut handlers = match handlers.entry(full_path.clone()) {
            hash_map::Entry::Occupied(entry) => {
                debug!("Adding new exiting watch");
                entry
            }
            hash_map::Entry::Vacant(entry) => {
                debug!("Spawning new watch");
                self.send(Ok(NotifyRequest::Watch {
                    full_path: full_path.clone(),
                }));
                entry.insert_entry(HashMap::new())
            }
        };
        handlers
            .get_mut()
            .insert(registration.id, Ptr::downgrade(&registration));
        return registration;
    }

    fn send(&self, notify_request: Result<NotifyRequest, ServerFnError>) {
        let mut request = self.inner(|inner| inner.request.clone());
        let send_task = async move {
            let () = request
                .send(notify_request)
                .await
                .unwrap_or_else(|error| warn!("Failed to send notify request: {error}"));
        };
        spawn_local(send_task.in_current_span());
    }
}

impl NotifyServiceImpl {
    #[autoclone]
    fn new(remote: Remote, inner: &Ptr<Mutex<Option<NotifyServiceImpl>>>) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded();
        let handlers = Handlers::default();
        let request = futures::stream::once(ready(Ok(NotifyRequest::Start {
            remote: remote.unwrap_or_default(),
        })))
        .chain(request_rx);
        #[cfg(debug_assertions)]
        let request = request.inspect(|r| debug!("Notify request: {r:?}"));
        let task = async move {
            autoclone!(inner, handlers);
            debug!("Start");
            defer!(debug!("End"));
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
                        let response_path = Path::new(&response.path);
                        let response_path_parent = response_path.parent();
                        let handlers = {
                            let lock = handlers.lock().unwrap();
                            (*lock).clone()
                        };
                        for (full_path, handlers) in handlers {
                            let full_path = full_path.as_deref().full_path();
                            for handler in handlers.values() {
                                let Some(handler) = handler.upgrade() else {
                                    continue;
                                };
                                if match handler.registration_type {
                                    RegistrationType::File => full_path == response_path,
                                    RegistrationType::Folder => {
                                        full_path == response_path
                                            || Some(full_path.as_ref()) == response_path_parent
                                    }
                                } {
                                    let callback = &*handler.callback;
                                    callback(&response)
                                }
                            }
                        }
                    }
                    Err(error) => {
                        warn!("{error:?}");
                        inner.lock().unwrap().take();
                        return;
                    }
                }
            }
        };
        spawn_local(task.in_current_span());
        Self {
            request: request_tx,
            handlers,
        }
    }
}

impl NotifyRegistration {
    fn new(
        full_path: FilePath<Arc<str>>,
        notify_service: &Ptr<NotifyService>,
        registration_type: RegistrationType,
        callback: impl Fn(&NotifyResponse) + 'static,
    ) -> Ptr<Self> {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, SeqCst);
        debug!(id, "Create notify registration");
        Ptr::new(NotifyRegistration {
            id,
            full_path,
            notify_service: Ptr::downgrade(notify_service),
            registration_type,
            callback: Box::new(callback),
        })
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
        {
            let handlers = notify_service.inner(|inner| inner.handlers.clone());
            trace!("Acquire lock");
            let mut handlers = handlers.lock().unwrap();
            trace!("Removing registration");
            let Some(handlers_by_id) = handlers.get_mut(&self.full_path) else {
                warn!("Registrations not found for {:?}", self.full_path);
                return;
            };

            handlers_by_id.remove(&self.id);
            if !handlers_by_id.is_empty() {
                return;
            }
            handlers.remove(&self.full_path);
        }
        notify_service.send(Ok(NotifyRequest::UnWatch {
            full_path: std::mem::take(&mut self.full_path),
        }));
    }
}
