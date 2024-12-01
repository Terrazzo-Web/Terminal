use std::rc::Rc;

use terminal_tab::TerminalTab;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::tabs::TabsDescriptor as _;
use tracing::debug;
use tracing::info;
use tracing::warn;
use wasm_bindgen_futures::spawn_local;

use self::terminal_tabs::TerminalTabs;
use crate::api;
use crate::terminal_id::TerminalId;
use crate::widgets::tabs::tabs;
use crate::widgets::tabs::TabsOptions;

stylance::import_crate_style!(style, "src/terminal.scss");

mod attach;
mod javascript;
mod terminal_tab;
mod terminal_tabs;

#[derive(Clone)]
pub struct TerminalsState {
    pub selected_tab: XSignal<TerminalId>,
    pub terminal_tabs: XSignal<TerminalTabs>,
}

pub fn terminals(template: XTemplate) {
    let terminal_id = TerminalId::from("Terminal");
    let selected_tab = XSignal::new("selected_tab", terminal_id.clone());
    let terminal_tabs = XSignal::new("terminal_tabs", TerminalTabs::from(Rc::new(vec![])));
    refresh_terminal_tabs(selected_tab.clone(), terminal_tabs.clone());
    let state = TerminalsState {
        selected_tab,
        terminal_tabs: terminal_tabs.clone(),
    };
    let consumers = render_terminals(template, state, terminal_tabs);
    std::mem::forget(consumers);
}

#[html]
#[template]
pub fn render_terminals(state: TerminalsState, #[signal] terminal_tabs: TerminalTabs) -> XElement {
    info!("Render terminals: {terminal_tabs:?}");
    div(
        class = style::terminals,
        tabs(
            terminal_tabs,
            state,
            Rc::new(TabsOptions {
                tabs_class: Some(style::tabs.into()),
                titles_class: Some(style::titles.into()),
                title_class: Some(style::title.into()),
                items_class: Some(style::items.into()),
                item_class: Some(style::item.into()),
                selected_class: Some(style::selected.into()),
                ..TabsOptions::default()
            }),
        ),
    )
}

fn refresh_terminal_tabs(selected_tab: XSignal<TerminalId>, terminal_tabs: XSignal<TerminalTabs>) {
    spawn_local(async move {
        let terminal_defs = match api::client::list::list().await {
            Ok(terminal_defs) => terminal_defs,
            Err(error) => {
                warn!("Failed to load terminal definitions: {error}");
                return;
            }
        };
        let batch = Batch::use_batch("refresh_terminal_tabs");
        if let Some(first_terminal) = terminal_defs.first() {
            let selected_tab_value = selected_tab.get_value_untracked();
            if !terminal_defs.iter().any(|def| def.id == selected_tab_value) {
                selected_tab.force(first_terminal.id.clone());
            }
        }
        terminal_tabs.set(TerminalTabs::from(Rc::new(
            terminal_defs
                .into_iter()
                .map(|def| TerminalTab::of(def, &selected_tab))
                .collect::<Vec<_>>(),
        )));
        drop(batch);
    });
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
        if let Some(last_dispatcher) = api::client::stream::drop_dispatcher(terminal_id) {
            spawn_local(api::client::stream::close_pipe(last_dispatcher));
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
        if next.id == *closed_terminal_id {
            if let Some(tab) = terminal_tabs.next() {
                return Some(tab.id.clone());
            } else {
                return prev;
            }
        }
        prev = Some(next.id.clone());
    }
    return prev;
}
