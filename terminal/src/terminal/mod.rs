#![cfg(feature = "client")]
#![cfg(feature = "terminal")]

use terminal_tab::TerminalTab;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::tabs::TabsDescriptor as _;
use terrazzo::widgets::tabs::TabsOptions;
use terrazzo::widgets::tabs::tabs;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use self::diagnostics::info;
use self::diagnostics::warn;
use self::terminal_tabs::TerminalTabs;
use crate::api::client::terminal_api;
use crate::terminal_id::TerminalId;

stylance::import_crate_style!(style, "src/terminal/terminal.scss");

mod attach;
mod javascript;
mod terminal_tab;
mod terminal_tabs;

#[derive(Clone)]
pub struct TerminalsState {
    pub selected_tab: XSignal<TerminalId>,
    pub terminal_tabs: XSignal<TerminalTabs>,
}

pub fn terminals(template: XTemplate) -> Consumers {
    let terminal_id = TerminalId::from("Terminal");
    let selected_tab = XSignal::new("selected-tab", terminal_id.clone());
    let terminal_tabs = XSignal::new("terminal-tabs", TerminalTabs::from(Ptr::new(vec![])));
    refresh_terminal_tabs(selected_tab.clone(), terminal_tabs.clone());
    let state = TerminalsState {
        selected_tab,
        terminal_tabs: terminal_tabs.clone(),
    };
    render_terminals(template, state, terminal_tabs)
}

#[html]
#[template]
pub fn render_terminals(state: TerminalsState, #[signal] terminal_tabs: TerminalTabs) -> XElement {
    info!("Render terminals: {terminal_tabs:?}");
    div(
        style = "height: 100%;",
        div(
            key = "terminals",
            class = style::terminals,
            tabs(
                terminal_tabs,
                state,
                Ptr::new(TabsOptions {
                    tabs_class: Some(style::tabs.into()),
                    titles_class: Some(style::titles.into()),
                    title_class: Some(style::title.into()),
                    items_class: Some(style::items.into()),
                    item_class: Some(style::item.into()),
                    selected_class: Some(style::selected.into()),
                    ..TabsOptions::default()
                }),
            ),
        ),
    )
}

fn refresh_terminal_tabs(selected_tab: XSignal<TerminalId>, terminal_tabs: XSignal<TerminalTabs>) {
    let refresh_terminal_tabs_task = async move {
        let terminal_defs = match terminal_api::list::list().await {
            Ok(terminal_defs) => terminal_defs,
            Err(error) => {
                warn!("Failed to load terminal definitions: {error}");
                return;
            }
        };
        let batch = Batch::use_batch("refresh_terminal_tabs");
        if let Some(first_terminal) = terminal_defs.first() {
            let selected_tab_value = selected_tab.get_value_untracked();
            if !terminal_defs
                .iter()
                .any(|def| def.address.id == selected_tab_value)
            {
                selected_tab.force(first_terminal.address.id.clone());
            }
        }
        terminal_tabs.set(TerminalTabs::from(Ptr::new(
            terminal_defs
                .into_iter()
                .map(|def| TerminalTab::of(def, &selected_tab))
                .collect::<Vec<_>>(),
        )));
        drop(batch);
    };
    spawn_local(refresh_terminal_tabs_task.in_current_span());
}

impl TerminalsState {
    /// Callback on end-of-stream to drop the terminal tab from the UI after the process is closed in the backend.
    pub fn on_eos(&self, terminal_id: &TerminalId) {
        debug!("Closing the terminal tab");
        let _batch = Batch::use_batch("close-tab");
        let TerminalsState {
            selected_tab,
            terminal_tabs,
        } = self;
        if selected_tab.get_value_untracked() == *terminal_id {
            if let Some(next_selected_tab) = next_selected_tab(terminal_tabs, terminal_id) {
                selected_tab.set(next_selected_tab);
            }
        }
        terminal_tabs.update(|terminal_tabs| Some(terminal_tabs.clone().remove_tab(terminal_id)));
        if let Some(last_dispatcher) = terminal_api::stream::drop_dispatcher(terminal_id) {
            spawn_local(terminal_api::stream::close_pipe(last_dispatcher).in_current_span());
        }
    }
}

fn next_selected_tab(
    terminal_tabs: &XSignal<TerminalTabs>,
    closed_terminal_id: &TerminalId,
) -> Option<TerminalId> {
    let terminal_tabs = terminal_tabs.get_value_untracked();
    let mut terminal_tabs = terminal_tabs.tab_descriptors().iter();
    let mut prev = None;
    while let Some(next) = terminal_tabs.next() {
        if next.address.id == *closed_terminal_id {
            if let Some(tab) = terminal_tabs.next() {
                return Some(tab.address.id.clone());
            } else {
                return prev;
            }
        }
        prev = Some(next.address.id.clone());
    }
    return prev;
}
