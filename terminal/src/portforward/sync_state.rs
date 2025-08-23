#![cfg(feature = "client")]

use bitflags::bitflags;
use scopeguard::guard;
use terrazzo::prelude::XSignal;

use crate::assets::icons;

#[derive(Clone, Copy, Debug, Default)]
pub struct SyncState {
    pending: Fields,
    loading: Fields,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Fields: u8 {
        const REMOTE = 1;
        const HOST = 1 << 1;
        const PORT = 1 << 2;
        const DELETE = 1 << 3;
        const ACTIVE = 1 << 4;
    }
}

impl SyncState {
    pub fn status_src(&self) -> icons::Icon {
        self.src(icons::port_forward_synchronized())
    }

    pub fn add_src(&self) -> icons::Icon {
        self.src(icons::add_port_forward())
    }

    fn src(&self, not_pending: icons::Icon) -> icons::Icon {
        if !self.loading.is_empty() {
            icons::port_forward_loading()
        } else if !self.pending.is_empty() {
            icons::port_forward_pending()
        } else {
            not_pending
        }
    }

    pub fn is_deleting(&self) -> bool {
        self.loading.contains(Fields::DELETE)
    }

    pub fn incr_pending(sync_state: XSignal<Self>, field: Fields) {
        sync_state.update(|sync_state| {
            Some(Self {
                pending: sync_state.pending | field,
                loading: sync_state.loading,
            })
        });
    }

    pub fn decr_pending(sync_state: XSignal<Self>, field: Fields) {
        sync_state.update(|sync_state| {
            Some(Self {
                pending: sync_state.pending - field,
                loading: sync_state.loading,
            })
        });
    }

    pub fn incr_loading(sync_state: XSignal<Self>, field: Fields) -> impl Drop {
        Self::incr_impl(
            sync_state,
            move |sync_state| Self {
                pending: sync_state.pending - field,
                loading: sync_state.loading | field,
            },
            move |sync_state| Self {
                pending: sync_state.pending,
                loading: sync_state.loading - field,
            },
        )
    }

    fn incr_impl(
        sync_state: XSignal<Self>,
        incr: impl FnOnce(&Self) -> Self,
        decr: impl FnOnce(&Self) -> Self,
    ) -> impl Drop {
        sync_state.update(|sync_state| Some(incr(sync_state)));
        return guard(sync_state, move |sync_state| {
            sync_state.update(|sync_state| Some(decr(sync_state)));
        });
    }
}
