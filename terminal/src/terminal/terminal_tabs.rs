use std::rc::Rc;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;
use tracing::info;
use tracing::warn;
use web_sys::MouseEvent;

use super::TerminalsState;
use super::terminal_tab::TerminalTab;
use crate::api;
use crate::api::client::remotes;
use crate::api::client_name::ClientName;
use crate::terminal_id::TerminalId;

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
                    active(t, client_names.clone())
                },
                img(src = "/static/icons/plus-square.svg"),
                click = move |_ev| {
                    wasm_bindgen_futures::spawn_local(async move {
                        autoclone!(this, state);
                        let terminal_def = match api::client::new_id::new_id(None).await {
                            Ok(id) => id,
                            Err(error) => {
                                warn!("Failed to allocate new ID: {error}");
                                return;
                            }
                        };
                        let new_tab = TerminalTab::new(terminal_def, &state.selected_tab);
                        let _batch = Batch::use_batch("add-tab");
                        state.selected_tab.force(new_tab.id.clone());
                        state.terminal_tabs.force(this.clone().add_tab(new_tab));
                    });
                },
            ),
            show_clients_dropdown(show_clients.clone(), client_names.clone()),
            mouseenter = move |_ev| {
                autoclone!(client_names, show_clients);
                show_clients.cancel();
                wasm_bindgen_futures::spawn_local(async move {
                    autoclone!(client_names);
                    let new_client_names = remotes::remotes()
                        .await
                        .or_else_throw(|error| format!("Failed to fetch remotes: {error}"));
                    client_names.set(Some(new_client_names));
                });
            },
            mouseleave = show_clients.wrap(move |_: MouseEvent| {
                autoclone!(client_names);
                client_names.set(None);
            }),
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
        move_tab(self.clone(), after_tab, moved_tab_key)
    }
}

fn move_tab(state: TerminalsState, after_tab: Option<TerminalTab>, moved_tab_key: String) {
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

#[autoclone]
#[html]
#[template(tag = ul)]
fn show_clients_dropdown(
    show_clients: Cancellable<Duration>,
    #[signal] client_names: Option<Vec<ClientName>>,
) -> XElement {
    info!("Render client names");
    if let Some(client_names) = client_names {
        let client_names = client_names.into_iter().map(|client_name| {
            li(
                "{client_name}",
                mouseenter = move |_ev| {
                    autoclone!(show_clients);
                    show_clients.cancel();
                },
            )
        });
        tag(class = style::add_client_tab, client_names..)
    } else {
        tag(style::visibility = "hidden", style::display = "none")
    }
}

#[template]
pub fn active(#[signal] client_names: Option<Vec<ClientName>>) -> XAttributeValue {
    client_names.map(|_| style::active)
}
