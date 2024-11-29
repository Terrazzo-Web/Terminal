use std::rc::Rc;

use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use tracing::warn;

use super::terminal_tab::TerminalTab;
use super::TerminalsState;
use crate::api;
use crate::terminal_id::TerminalId;
use crate::widgets::tabs::TabsDescriptor;
use crate::widgets::tabs::TabsState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalTabs {
    terminal_tabs: Rc<Vec<TerminalTab>>,
}

impl From<Rc<Vec<TerminalTab>>> for TerminalTabs {
    fn from(terminal_tabs: Rc<Vec<TerminalTab>>) -> Self {
        Self { terminal_tabs }
    }
}

impl TabsDescriptor for TerminalTabs {
    type TabDescriptor = TerminalTab;
    type State = TerminalsState;

    fn tab_descriptors(&self) -> &[Self::TabDescriptor] {
        &self.terminal_tabs
    }

    #[html]
    fn after_titles(&self, state: &TerminalsState) -> impl IntoIterator<Item = impl Into<XNode>> {
        let this = self.clone();
        let state = state.clone();
        [div(
            class = super::style::add_tab_icon,
            key = "add-tab-icon",
            div(
                img(src = "/static/icons/plus-square.svg"),
                click = move |_ev| {
                    let this = this.clone();
                    let state = state.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let terminal_id = match api::client::new_id::new_id().await {
                            Ok(id) => id,
                            Err(error) => {
                                warn!("Failed to allocate new ID: {error}");
                                return;
                            }
                        };
                        let new_tab = TerminalTab::new(terminal_id.into(), &state.selected_tab);
                        let _batch = Batch::use_batch("add-tab");
                        state.selected_tab.force(new_tab.id.clone());
                        state.terminal_tabs.force(this.clone().add_tab(new_tab));
                    });
                },
            ),
        )]
    }
}

impl TerminalTabs {
    pub fn add_tab(mut self, new: TerminalTab) -> Self {
        let terminal_tabs = Rc::make_mut(&mut self.terminal_tabs);
        terminal_tabs.push(new);
        self
    }

    pub fn remove_tab(mut self, id: &TerminalId) -> Self {
        let terminal_tabs = Rc::make_mut(&mut self.terminal_tabs);
        terminal_tabs.retain(|tab| tab.id != *id);
        self
    }
}

impl TabsState for TerminalsState {
    type TabDescriptor = TerminalTab;

    fn move_tab(&self, after_tab: Option<TerminalTab>, moved_tab_key: String) {
        self.terminal_tabs.update(|TerminalTabs { terminal_tabs }| {
            let after_tab = if let Some(after_tab) = after_tab {
                terminal_tabs.iter().find(|tab| tab.id == after_tab.id)
            } else {
                None
            };
            let moved_tab = terminal_tabs
                .iter()
                .find(|tab| tab.id.as_str() == moved_tab_key)
                .unwrap();
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
            self.selected_tab.set(moved_tab.id.clone());
            return Some(TerminalTabs {
                terminal_tabs: Rc::new(tabs),
            });
        });
    }
}
