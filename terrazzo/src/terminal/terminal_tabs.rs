use std::rc::Rc;

use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use tracing::warn;

use super::terminal_tab::TerminalTab;
use super::TerminalsState;
use crate::api;
use crate::terminal_id::TerminalId;
use crate::widgets::tabs::TabsDescriptor;

#[derive(Clone, Debug)]
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
                img(src = "/assets/icons/plus-square.svg"),
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
