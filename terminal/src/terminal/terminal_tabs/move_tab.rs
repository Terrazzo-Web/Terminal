use terrazzo::prelude::*;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::Instrument as _;
use self::diagnostics::warn;
use super::TerminalTabs;
use crate::api;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

pub fn move_tab(state: TerminalsState, after_tab: Option<TerminalTab>, moved_tab_key: String) {
    let tabs = state
        .terminal_tabs
        .update(|TerminalTabs { terminal_tabs }| {
            let after_tab = if let Some(after_tab) = after_tab {
                terminal_tabs
                    .iter()
                    .find(|tab| tab.address.id == after_tab.address.id)
            } else {
                None
            };
            let moved_tab = terminal_tabs
                .iter()
                .find(|tab| tab.address.id.as_str() == moved_tab_key)
                .or_throw("'moved_tab' not found");
            let tabs = terminal_tabs
                .iter()
                .enumerate()
                .flat_map(|(i, tab)| {
                    if after_tab.is_some_and(|t| tab.address.id == t.address.id) {
                        [Some(tab), Some(moved_tab)]
                    } else if after_tab.is_none() && i == 0 {
                        [Some(moved_tab), Some(tab)]
                    } else if tab.address.id == moved_tab.address.id {
                        Default::default()
                    } else {
                        [Some(tab), None]
                    }
                })
                .flatten()
                .filter({
                    // Handle move to same position
                    let mut last = None;
                    move |tab| {
                        let result = Some(&tab.address.id) != last.as_ref();
                        last = Some(tab.address.id.clone());
                        return result;
                    }
                })
                .cloned()
                .collect();
            state.selected_tab.set(moved_tab.address.id.clone());
            let tabs = TerminalTabs {
                terminal_tabs: Ptr::new(tabs),
            };
            return Some(tabs.clone()).and_return(tabs);
        });
    let tabs = tabs
        .terminal_tabs
        .iter()
        .map(|tab| tab.address.clone())
        .collect();
    let set_order_task = async move {
        let () = api::client::set_order::set_order(tabs)
            .await
            .unwrap_or_else(|error| warn!("Failed to set order: {error}"));
    };
    spawn_local(set_order_task.in_current_span());
}
