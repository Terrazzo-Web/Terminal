#![cfg(feature = "client")]

use scopeguard::guard;
use terrazzo::prelude::XSignal;

use crate::assets::icons;

#[derive(Clone, Copy, Debug, Default)]
pub struct SyncState {
    pending: u16,
    loading: u16,
}

impl SyncState {
    pub fn src(&self) -> icons::Icon {
        if self.pending > 0 {
            icons::port_forward_pending()
        } else if self.loading > 0 {
            icons::port_forward_loading()
        } else {
            icons::port_forward_synchronized()
        }
    }

    pub fn incr_pending(sync_state: XSignal<Self>) -> impl Drop {
        Self::incr_impl(
            sync_state,
            |sync_state| Self {
                pending: sync_state.pending + 1,
                loading: sync_state.loading,
            },
            |sync_state| Self {
                pending: sync_state.pending - 1,
                loading: sync_state.loading,
            },
        )
    }

    pub fn incr_loading(sync_state: XSignal<Self>) -> impl Drop {
        Self::incr_impl(
            sync_state,
            |sync_state| Self {
                pending: sync_state.pending,
                loading: sync_state.loading + 1,
            },
            |sync_state| Self {
                pending: sync_state.pending,
                loading: sync_state.loading - 1,
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
