use std::rc::Rc;

use terrazzo::prelude::*;
use tracing::warn;

use super::TerminalTabs;
use crate::api;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

pub fn move_tab(state: TerminalsState, after_tab: Option<TerminalTab>, moved_tab_key: String) {
    let tabs = state
        .terminal_tabs
        .update(|TerminalTabs { terminal_tabs }| {
            let after_tab = if let Some(after_tab) = after_tab {
                terminal_tabs.iter().find(|tab| tab.id == after_tab.id)
            } else {
                None
            };
            let moved_tab = terminal_tabs
                .iter()
                .find(|tab| tab.id.as_str() == moved_tab_key)
                .or_throw("'moved_tab' not found");
            let tabs = terminal_tabs
                .iter()
                .enumerate()
                .flat_map(|(i, tab)| {
                    if after_tab.is_some_and(|t| tab.id == t.id) {
                        [Some(tab), Some(moved_tab)]
                    } else if after_tab.is_none() && i == 0 {
                        [Some(moved_tab), Some(tab)]
                    } else if tab.id == moved_tab.id {
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
                        let result = Some(&tab.id) != last.as_ref();
                        last = Some(tab.id.clone());
                        return result;
                    }
                })
                .cloned()
                .collect();
            state.selected_tab.set(moved_tab.id.clone());
            let tabs = TerminalTabs {
                terminal_tabs: Rc::new(tabs),
            };
            return Some(tabs.clone()).and_return(tabs);
        });
    let tabs = tabs
        .terminal_tabs
        .iter()
        .map(|tab| tab.id.clone())
        .collect();
    wasm_bindgen_futures::spawn_local(async move {
        let () = api::client::set_order::set_order(tabs)
            .await
            .unwrap_or_else(|error| warn!("Failed to set order: {error}"));
    });
}
