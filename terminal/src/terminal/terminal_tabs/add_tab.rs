use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use tracing::debug;
use tracing::warn;
use web_sys::MouseEvent;

use crate::api;
use crate::api::client_address::ClientAddress;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

#[derive(Clone)]
pub struct RemotesState {
    pub(super) remotes: XSignal<Remotes>,
    pub(super) show_remotes: Cancellable<()>,
    pub(super) hide_remotes: Cancellable<Duration>,
}

pub type Remotes = Option<Vec<ClientAddress>>;

impl RemotesState {
    pub fn new() -> Self {
        Self {
            remotes: XSignal::new("remotes", None),
            show_remotes: Cancellable::new(),
            hide_remotes: Duration::from_millis(250).cancellable(),
        }
    }
}

#[autoclone]
#[html]
#[template(tag = ul)]
pub fn show_clients_dropdown(
    state: TerminalsState,
    #[signal] remotes: Remotes,
    hide_clients: Cancellable<Duration>,
) -> XElement {
    debug!("Render client names");
    if let Remotes::Some(remotes) = remotes {
        if !remotes.is_empty() {
            let client_names = remotes.into_iter().map(|client_address| {
                li(
                    "{client_address} ⏎",
                    mouseenter = move |_ev| {
                        autoclone!(hide_clients);
                        hide_clients.cancel();
                    },
                    click = create_terminal(state.clone(), client_address),
                )
            });
            return tag(class = super::style::add_client_tab, client_names..);
        }
    }
    return tag(style::visibility = "hidden", style::display = "none");
}

impl RemotesState {
    #[autoclone]
    pub fn mouseenter(&self) -> impl Fn(MouseEvent) + 'static {
        let client_names_state = self.clone();
        move |_| {
            let Self {
                remotes,
                show_remotes,
                hide_remotes,
            } = &client_names_state;
            show_remotes.cancel();

            let update_remotes = show_remotes.capture(move |new_remotes| {
                autoclone!(remotes);
                remotes.set(new_remotes)
            });
            hide_remotes.cancel();
            wasm_bindgen_futures::spawn_local(async move {
                let remotes = api::client::remotes::remotes()
                    .await
                    .or_else_throw(|error| format!("Failed to fetch remotes: {error}"));
                if update_remotes(remotes).is_none() {
                    debug!("Updating remotes was canceled");
                }
            });
        }
    }

    #[autoclone]
    pub fn mouseleave(&self) -> impl Fn(MouseEvent) + 'static {
        let Self {
            remotes,
            hide_remotes,
            ..
        } = self;
        hide_remotes.wrap(move |_| {
            autoclone!(remotes);
            remotes.set(Remotes::None);
        })
    }
}

#[template]
pub fn active(#[signal] remotes: Remotes) -> XAttributeValue {
    if let Remotes::Some { .. } = remotes {
        Some(super::style::active)
    } else {
        None
    }
}

#[autoclone]
pub fn create_terminal(
    state: TerminalsState,
    client_address: ClientAddress,
) -> impl Fn(MouseEvent) {
    move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            autoclone!(state, client_address);
            let terminal_def = match api::client::new_id::new_id(client_address.clone()).await {
                Ok(id) => id,
                Err(error) => {
                    warn!("Failed to allocate new ID: {error}");
                    return;
                }
            };
            let new_tab = TerminalTab::new(terminal_def, &state.selected_tab);
            let _batch = Batch::use_batch("add-tab");
            state.selected_tab.force(new_tab.address.id.clone());
            state
                .terminal_tabs
                .update(|tabs| Some(tabs.clone().add_tab(new_tab)));
        });
    }
}
