#![cfg(feature = "client")]

use std::num::NonZero;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::warn;

use super::style;
use crate::assets::icons;

/// State shows a spinner when the file is being saved.
#[derive(Clone, Copy, Debug)]
pub enum SynchronizedState {
    Sync,
    Pending(NonZero<u32>),
}

#[html]
#[template(tag = img)]
pub fn show_synchronized_state(synchronized_state: XSignal<SynchronizedState>) -> XElement {
    tag(
        class = style::sync_status,
        src %= move |t| icon_src(t, synchronized_state.clone()),
    )
}

#[template]
fn icon_src(#[signal] synchronized_state: SynchronizedState) -> XAttributeValue {
    match synchronized_state {
        SynchronizedState::Sync => icons::done(),
        SynchronizedState::Pending { .. } => icons::loading(),
    }
}

impl SynchronizedState {
    pub fn enqueue(state: XSignal<SynchronizedState>) -> impl Drop {
        state.update(|state| {
            Some(match state {
                Self::Sync => Self::Pending(NonZero::<u32>::MIN),
                Self::Pending(c) => Self::Pending(c.saturating_add(1)),
            })
        });
        scopeguard::guard(state, |state| {
            state.update(|state| {
                Some(match state {
                    Self::Sync => {
                        warn!("Impossible state");
                        Self::Sync
                    }
                    Self::Pending(c) => (c.get() - 1)
                        .try_into()
                        .map(Self::Pending)
                        .unwrap_or(SynchronizedState::Sync),
                })
            });
        })
    }
}
