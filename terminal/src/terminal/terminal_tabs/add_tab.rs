use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use tracing::info;
use tracing::warn;
use web_sys::MouseEvent;

use super::TerminalTabs;
use crate::api;
use crate::api::client::remotes;
use crate::api::client_name::ClientName;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

#[autoclone]
#[html]
#[template(tag = ul)]
pub fn show_clients_dropdown(
    #[signal] client_names: Option<Vec<ClientName>>,
    show_clients: Cancellable<Duration>,
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
        tag(class = super::style::add_client_tab, client_names..)
    } else {
        tag(style::visibility = "hidden", style::display = "none")
    }
}

#[autoclone]
pub fn mouseenter(
    client_names: &XSignal<Option<Vec<ClientName>>>,
    show_clients: &Cancellable<Duration>,
) {
    show_clients.cancel();
    wasm_bindgen_futures::spawn_local(async move {
        autoclone!(client_names);
        let new_client_names = remotes::remotes()
            .await
            .or_else_throw(|error| format!("Failed to fetch remotes: {error}"));
        client_names.set(Some(new_client_names));
    });
}

pub fn mouseleave(client_names: &XSignal<Option<Vec<ClientName>>>) {
    client_names.set(None);
}

#[template]
pub fn active(#[signal] client_names: Option<Vec<ClientName>>) -> XAttributeValue {
    client_names.map(|_| super::style::active)
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
