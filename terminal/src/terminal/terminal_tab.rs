use std::ops::Deref;
use std::rc::Rc;
use std::sync::Mutex;

use nameth::NamedType;
use nameth::nameth;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::editable::editable;
use terrazzo::widgets::tabs::TabDescriptor;
use tracing::Level;
use tracing::debug;
use tracing::warn;

use super::TerminalsState;
use super::attach;
use super::javascript::TerminalJs;
use super::style;
use crate::api;
use crate::api::TabTitle;
use crate::api::TerminalDef;
use crate::api::client::LiveTerminalDef;
use crate::terminal_id::TerminalId;

#[nameth]
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalTab(Rc<TerminalTabInner>);

pub struct TerminalTabInner {
    def: LiveTerminalDef,
    pub selected: XSignal<bool>,
    pub xtermjs: Mutex<Option<TerminalJs>>,
    #[expect(unused)]
    registrations: Consumers,
}

impl TerminalTab {
    pub fn new(terminal_def: TerminalDef, selected: &XSignal<TerminalId>) -> Self {
        Self::of(terminal_def, selected)
    }

    #[autoclone]
    pub fn of(terminal_definition: TerminalDef, selected: &XSignal<TerminalId>) -> Self {
        let TerminalDef {
            address,
            title,
            order,
        } = terminal_definition;
        let terminal_id = &address.id;
        let selected = {
            let name: XString = if tracing::enabled!(Level::DEBUG) {
                format!("is_selected_tab:{terminal_id}").into()
            } else {
                "is_selected_tab".into()
            };
            selected.derive(
                name,
                move |selected_tab| {
                    autoclone!(terminal_id);
                    return selected_tab == &terminal_id;
                },
                if_change(move |_, is_selected: &bool| {
                    autoclone!(terminal_id);
                    is_selected.then(|| terminal_id.clone())
                }),
            )
        };
        let title = {
            let signal_name: XString = if tracing::enabled!(Level::DEBUG) {
                format!("terminal_title:{terminal_id}").into()
            } else {
                "terminal_title".into()
            };
            XSignal::new(signal_name, title.map(XString::from))
        };
        let registrations = title.add_subscriber(move |title: TabTitle<XString>| {
            autoclone!(address);
            wasm_bindgen_futures::spawn_local(async move {
                autoclone!(address);
                let result =
                    api::client::set_title::set_title(&address, title.map(|t| t.to_string()));
                if let Err(error) = result.await {
                    warn!("Failed to update title: {error}")
                }
            });
        });
        Self(Rc::new(TerminalTabInner {
            def: LiveTerminalDef {
                address,
                title,
                order,
            },
            selected,
            xtermjs: Mutex::new(None),
            registrations,
        }))
    }
}

impl TabDescriptor for TerminalTab {
    type State = TerminalsState;

    fn key(&self) -> XString {
        self.address.id.clone().into()
    }

    #[autoclone]
    #[html]
    fn title(&self, state: &TerminalsState) -> impl Into<XNode> {
        let terminal = &self.address;
        let id = &terminal.id;
        let title = self.title.derive(
            "resolve_title",
            |t| t.override_title.as_ref().unwrap_or(&t.shell_title).clone(),
            if_change(|old: &TabTitle<XString>, new: &XString| {
                Some(TabTitle {
                    shell_title: old.shell_title.clone(),
                    override_title: if new.is_empty() {
                        // Override to empty means fallback to shell title.
                        None
                    } else if *new == old.shell_title && old.override_title.is_none() {
                        // Can't override to shell title.
                        None
                    } else {
                        // Set the override.
                        Some(new.clone())
                    },
                })
            }),
        );
        let selected_tab = state.selected_tab.clone();
        let title_link = span(move |template| {
            autoclone!(id, title, selected_tab);
            print_editable_title(template, id.clone(), title.clone(), selected_tab.clone())
        });
        let close_button = img(
            key = "close-icon",
            class = super::style::close_icon,
            src = "/static/icons/x-lg.svg",
            click = move |ev: web_sys::MouseEvent| {
                autoclone!(terminal);
                ev.stop_propagation();
                wasm_bindgen_futures::spawn_local(async move {
                    autoclone!(terminal);
                    api::client::stream::try_restart_pipe();
                    api::client::stream::close(&terminal, None).await;
                });
            },
        );

        div([title_link, close_button]..)
    }

    #[html]
    fn item(&self, state: &TerminalsState) -> impl Into<XNode> {
        let this = self.clone();
        let state = state.clone();
        div(
            class = style::terminal,
            div(move |template| attach::attach(template, state.clone(), this.clone())),
        )
    }

    fn selected(&self, _state: &TerminalsState) -> XSignal<bool> {
        self.selected.clone()
    }
}

impl Deref for TerminalTab {
    type Target = TerminalTabInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for TerminalTabInner {
    type Target = LiveTerminalDef;

    fn deref(&self) -> &Self::Target {
        &self.def
    }
}

impl TerminalTabInner {
    pub fn to_terminal_def(&self) -> TerminalDef {
        TerminalDef {
            address: self.address.clone(),
            title: self.title.get_value_untracked().map(|t| t.to_string()),
            order: self.order,
        }
    }
}

#[autoclone]
#[html]
#[template]
fn print_editable_title(
    terminal_id: TerminalId,
    title: XSignal<XString>,
    selected_tab: XSignal<TerminalId>,
) -> XElement {
    let editing = XSignal::new("Editing", false);
    let is_editable = selected_tab.view("is_editable", move |selected_tab| {
        autoclone!(terminal_id);
        *selected_tab == terminal_id
    });
    span(move |t| {
        editable(
            t,
            title.clone(),
            is_editable.clone(),
            editing.clone(),
            move || {
                autoclone!(terminal_id, title);
                [span(move |t| {
                    terrazzo::widgets::link::link(
                        t,
                        move |_ev| {
                            autoclone!(terminal_id);
                            debug!("Clicks selected on terminal_id:{terminal_id}");
                        },
                        move || {
                            autoclone!(title);
                            [span(move |t| print_title(t, title.clone()))]
                        },
                    )
                })]
            },
        )
    })
}

#[html]
#[template]
fn print_title(#[signal] title: XString) -> XElement {
    span("{title}")
}

impl std::fmt::Debug for TerminalTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(TerminalTab::type_name())
            .field(&self.address.id.as_str())
            .finish()
    }
}

impl PartialEq for TerminalTabInner {
    fn eq(&self, other: &Self) -> bool {
        self.address.id == other.address.id
    }
}

impl Eq for TerminalTabInner {}
