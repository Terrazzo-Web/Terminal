use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use tracing::info;
use tracing::warn;
use web_sys::MouseEvent;

use super::TerminalTabs;
use crate::api;
use crate::api::client::remotes;
use crate::api::client_name::ClientName;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

#[derive(Clone)]
pub struct ClientNamesState {
    pub client_names: XSignal<ClientNames>,
    pub hide_clients: Cancellable<Duration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientNames {
    None,
    Pending,
    Some(Vec<ClientName>),
}

impl ClientNamesState {
    pub fn new() -> Self {
        Self {
            client_names: XSignal::new("client_names", ClientNames::None),
            hide_clients: Duration::from_millis(250).cancellable(),
        }
    }
}

#[autoclone]
#[html]
#[template(tag = ul)]
pub fn show_clients_dropdown(
    #[signal] client_names: ClientNames,
    hide_clients: Cancellable<Duration>,
) -> XElement {
    info!("Render client names");
    if let ClientNames::Some(client_names) = client_names {
        let client_names = client_names.into_iter().map(|client_name| {
            li(
                "{client_name}",
                mouseenter = move |_ev| {
                    autoclone!(hide_clients);
                    hide_clients.cancel();
                },
            )
        });
        tag(class = super::style::add_client_tab, client_names..)
    } else {
        tag(style::visibility = "hidden", style::display = "none")
    }
}

impl ClientNamesState {
    #[autoclone]
    pub fn mouseenter(&self) -> impl Fn(MouseEvent) + 'static {
        let client_names_state = self.clone();
        move |_| {
            let Self {
                client_names,
                hide_clients,
            } = &client_names_state;
            hide_clients.cancel();
            client_names.set(ClientNames::Pending);
            wasm_bindgen_futures::spawn_local(async move {
                autoclone!(client_names);
                let new_client_names = remotes::remotes()
                    .await
                    .or_else_throw(|error| format!("Failed to fetch remotes: {error}"));
                client_names.update(|old| {
                    if let ClientNames::Pending = old {
                        Some(ClientNames::Some(new_client_names))
                    } else {
                        None
                    }
                })
            });
        }
    }

    #[autoclone]
    pub fn mouseleave(&self) -> impl Fn(MouseEvent) + 'static {
        let Self {
            client_names,
            hide_clients,
        } = self;
        hide_clients.wrap(move |_| {
            autoclone!(client_names);
            client_names.set(ClientNames::None);
        })
    }
}

#[template]
pub fn active(#[signal] client_names: ClientNames) -> XAttributeValue {
    if let ClientNames::Some { .. } = client_names {
        Some(super::style::active)
    } else {
        None
    }
}

#[autoclone]
pub fn create_terminal(
    tabs: TerminalTabs,
    state: TerminalsState,
    client_name: Option<ClientName>,
) -> impl Fn(MouseEvent) {
    move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            autoclone!(tabs, state, client_name);
            let terminal_def = match api::client::new_id::new_id(client_name.clone()).await {
                Ok(id) => id,
                Err(error) => {
                    warn!("Failed to allocate new ID: {error}");
                    return;
                }
            };
            let new_tab = TerminalTab::new(terminal_def, &state.selected_tab);
            let _batch = Batch::use_batch("add-tab");
            state.selected_tab.force(new_tab.id.clone());
            state.terminal_tabs.force(tabs.clone().add_tab(new_tab));
        });
    }
}
