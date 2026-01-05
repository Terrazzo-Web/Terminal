use terrazzo::autoclone;
use terrazzo::prelude::*;
use terrazzo::template;

use self::diagnostics::Instrument as _;
use self::diagnostics::warn;
use crate::api::client::terminal_api;
use crate::api::client_address::ClientAddress;
use crate::frontend::remotes::Remotes;
use crate::terminal::TerminalsState;
use crate::terminal::terminal_tab::TerminalTab;

#[template(wrap = true)]
pub fn active(#[signal] remotes: Remotes) -> XAttributeValue {
    if let Remotes::Some { .. } = remotes {
        Some(super::style::active)
    } else {
        None
    }
}

#[autoclone]
pub fn create_terminal(state: TerminalsState, client_address: ClientAddress) {
    let task = async move {
        autoclone!(state, client_address);
        let terminal_def = match terminal_api::new_id::new_id(client_address.clone()).await {
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
    };
    wasm_bindgen_futures::spawn_local(task.in_current_span());
}
