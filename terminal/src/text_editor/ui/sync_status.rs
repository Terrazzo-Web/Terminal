use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::assets::icons;
use crate::text_editor::ui::SynchronizedState;

#[html]
#[template(tag = img)]
pub fn sync_status_icon(synchronized_state: XSignal<SynchronizedState>) -> XElement {
    tag(
        class = super::style::sync_status,
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
