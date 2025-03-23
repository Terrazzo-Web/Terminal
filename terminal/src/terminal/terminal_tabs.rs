use std::rc::Rc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::widgets::debounce::DoDebounce as _;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;

use super::TerminalsState;
use super::terminal_tab::TerminalTab;
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

    #[autoclone]
    #[html]
    fn after_titles(&self, state: &TerminalsState) -> impl IntoIterator<Item = impl Into<XNode>> {
        let this = self.clone();
        let state = state.clone();
        let client_names = XSignal::new("client_names", None);
        let show_clients = std::time::Duration::from_millis(250).cancellable();
        [div(
            class = style::add_tab_icon,
            key = "add-tab-icon",
            div(
                class %= move |t| {
                    autoclone!(client_names);
                    add_tab::active(t, client_names.clone())
                },
                img(src = "/static/icons/plus-square.svg"),
                click = add_tab::create_terminal(this.clone(), state.clone(), None),
                mouseenter = move |_| {
                    autoclone!(client_names, show_clients);
                    add_tab::mouseenter(&client_names, &show_clients);
                },
            ),
            mouseleave = show_clients.wrap(move |_| {
                autoclone!(client_names);
                add_tab::mouseleave(&client_names);
            }),
            add_tab::show_clients_dropdown(client_names.clone(), show_clients.clone()),
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
        move_tab::move_tab(self.clone(), after_tab, moved_tab_key)
    }
}
