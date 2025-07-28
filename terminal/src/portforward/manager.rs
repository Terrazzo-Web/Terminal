#![cfg(feature = "client")]

use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

use terrazzo::autoclone;
use terrazzo::prelude::XSignal;
use terrazzo::prelude::diagnostics;
use wasm_bindgen_futures::spawn_local;

use self::inner::ManagerImpl;
use super::schema::PortForward;
use super::sync_state::SyncState;
use crate::api::client::remotes_api;
use crate::api::client_address::ClientAddress;
use crate::frontend::remotes::Remote;

#[derive(Clone)]
pub struct Manager(Arc<ManagerImpl>);

mod inner {
    use std::sync::Arc;
    use std::sync::Mutex;

    use terrazzo::prelude::XSignal;

    use crate::api::client_address::ClientAddress;
    use crate::frontend::remotes::Remote;
    use crate::portforward::schema::PortForward;

    pub struct ManagerImpl {
        pub(super) port_forwards_signal: XSignal<Arc<[PortForward]>>,
        pub(super) port_forwards: Mutex<Arc<[PortForward]>>,
        pub(super) remote: XSignal<Remote>,
        pub(super) remotes: XSignal<Vec<ClientAddress>>,
    }
}

impl Deref for Manager {
    type Target = ManagerImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Manager {
    #[autoclone]
    pub fn new() -> Self {
        let manager = Self(Arc::new(ManagerImpl {
            port_forwards_signal: XSignal::new("port-forwards", Default::default()),
            port_forwards: Mutex::default(),
            remote: XSignal::new("remote", Remote::default()),
            remotes: XSignal::new("remotes", vec![]),
        }));

        // TODO: Show the remotes accessible from the selected remote.
        spawn_local(async move {
            autoclone!(manager);
            let Ok(remotes) = remotes_api::remotes().await else {
                return;
            };
            manager.remotes.set(remotes);
        });
        return manager;
    }

    pub fn remote(&self) -> XSignal<Remote> {
        self.remote.clone()
    }
    pub fn remotes(&self) -> XSignal<Vec<ClientAddress>> {
        self.remotes.clone()
    }
    pub fn port_forwards(&self) -> XSignal<Arc<[PortForward]>> {
        self.port_forwards_signal.clone()
    }
    pub fn port_forwards_lock(&self) -> std::sync::MutexGuard<Arc<[PortForward]>> {
        self.port_forwards.lock().expect("port_forwards lock")
    }

    pub fn load_port_forwards(&self, remote: Remote) {
        let manager = self.clone();
        spawn_local(async move {
            let Ok(port_forwards) = super::state::load_port_forwards(remote).await else {
                return;
            };
            *manager.port_forwards_lock() = port_forwards.clone();
            manager.port_forwards_signal.set(port_forwards);
        });
    }

    pub fn set(
        &self,
        remote: &Remote,
        sync_state: XSignal<SyncState>,
        id: i32,
        update_fn: impl FnOnce(&PortForward) -> Option<PortForward>,
    ) {
        let mut update_fn = Some(update_fn);
        self.update(remote, sync_state, move |port_forwards| {
            port_forwards
                .iter()
                .filter_map(|port_forward| {
                    if port_forward.id == id {
                        let update_fn = update_fn.take().unwrap();
                        update_fn(port_forward)
                    } else {
                        Some(port_forward.clone())
                    }
                })
                .collect::<Vec<_>>()
                .into()
        });
    }

    #[autoclone]
    pub fn update(
        &self,
        remote: &Remote,
        sync_state: XSignal<SyncState>,
        update_fn: impl FnOnce(&Arc<[PortForward]>) -> Arc<[PortForward]>,
    ) {
        let loading = SyncState::incr_loading(sync_state);
        let mut port_forwards_lock = self.port_forwards_lock();
        let new = update_fn(&port_forwards_lock);
        *port_forwards_lock = new.clone();
        drop(port_forwards_lock);

        let this = self.clone();
        spawn_local(async move {
            autoclone!(remote);
            let Ok(()) = super::state::store_port_forwards(remote, new.clone())
                .await
                .inspect_err(|error| diagnostics::warn!("Failed to save port forwards: {error}"))
            else {
                return;
            };
            this.port_forwards_signal.set(new);
            drop(loading);
        })
    }
}
