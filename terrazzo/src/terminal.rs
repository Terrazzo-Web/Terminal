use std::rc::Rc;

use terminal_tab::TerminalTab;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use tracing::info;
use tracing::warn;

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
    std::mem::forget(state.clone());
    let consumers = render_terminals(template, state, terminal_tabs);
    std::mem::forget(consumers);
}

#[template]
#[html]
pub fn render_terminals(state: TerminalsState, #[signal] terminal_tabs: TerminalTabs) -> XElement {
    info!("Render terminals: {terminal_tabs:?}");
    div(
        class = style::terminals,
        div(move |template| {
            let state = state.clone();
            tabs(
                template,
                terminal_tabs.clone(),
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
            )
        }),
    )
}

fn refresh_terminal_tabs(selected_tab: XSignal<TerminalId>, terminal_tabs: XSignal<TerminalTabs>) {
    wasm_bindgen_futures::spawn_local(async move {
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
        terminal_tabs.force(TerminalTabs::from(Rc::new(
            terminal_defs
                .into_iter()
                .map(|def| TerminalTab::of(def, &selected_tab))
                .collect::<Vec<_>>(),
        )));
        drop(batch);
    });
}
