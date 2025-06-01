use std::rc::Rc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;

use self::add_tab::RemotesState;
use super::TerminalsState;
use super::terminal_tab::TerminalTab;
use crate::api::client_address::ClientAddress;
use crate::assets::icons;
use crate::frontend::menu::menu;
use crate::terminal_id::TerminalId;

mod add_tab;
mod move_tab;

stylance::import_crate_style!(style, "src/terminal/terminal_tabs.scss");

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
    fn before_titles(&self, _state: &TerminalsState) -> impl IntoIterator<Item = impl Into<XNode>> {
        Some(menu())
    }

    #[autoclone]
    #[html]
    fn after_titles(&self, state: &TerminalsState) -> impl IntoIterator<Item = impl Into<XNode>> {
        let client_names_state = RemotesState::new();
        [div(
            class = style::add_tab_icon,
            key = "add-tab-icon",
            div(
                class %= move |t| {
                    autoclone!(client_names_state);
                    add_tab::active(t, client_names_state.remotes.clone())
                },
                img(src = icons::add_tab()),
                click = add_tab::create_terminal(state.clone(), ClientAddress::default()),
                mouseenter = client_names_state.mouseenter(),
            ),
            mouseleave = client_names_state.mouseleave(),
            add_tab::show_clients_dropdown(
                state.clone(),
                client_names_state.remotes.clone(),
                client_names_state.hide_remotes.clone(),
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
        terminal_tabs.retain(|tab| tab.address.id != *id);
        self
    }
}

impl TabsState for TerminalsState {
    type TabDescriptor = TerminalTab;

    fn move_tab(&self, after_tab: Option<TerminalTab>, moved_tab_key: String) {
        move_tab::move_tab(self.clone(), after_tab, moved_tab_key)
    }
}
