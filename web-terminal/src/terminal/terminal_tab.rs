use std::ops::Deref;
use std::rc::Rc;
use std::sync::Mutex;

use named::named;
use named::NamedType;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use tracing::debug;
use tracing::warn;
use tracing::Level;

use super::attach;
use super::javascript::TerminalJs;
use super::style;
use super::TerminalsState;
use crate::api;
use crate::api::TerminalDef;
use crate::terminal_id::TerminalId;
use crate::widgets;
use crate::widgets::editable::editable;
use crate::widgets::tabs::TabDescriptor;

#[named]
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalTab(Rc<TerminalTabInner>);

pub struct TerminalTabInner {
    pub id: TerminalId,
    pub title: XSignal<XString>,
    pub selected: XSignal<bool>,
    pub xtermjs: Mutex<Option<TerminalJs>>,
    #[expect(unused)]
    registrations: Consumers,
}

impl TerminalTab {
    pub fn new(terminal_id: TerminalId, selected: &XSignal<TerminalId>) -> Self {
        Self::of(
            TerminalDef {
                id: terminal_id.clone(),
                title: terminal_id.to_string(),
            },
            selected,
        )
    }

    #[autoclone]
    pub fn of(terminal_definition: TerminalDef, selected: &XSignal<TerminalId>) -> Self {
        let TerminalDef {
            id: terminal_id,
            title: terminal_title,
        } = terminal_definition;
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
                format!("title:{terminal_id}").into()
            } else {
                "terminal_title".into()
            };
            XSignal::new(signal_name, terminal_title.into())
        };
        let registrations = title.add_subscriber(move |title: XString| {
            autoclone!(terminal_id);
            wasm_bindgen_futures::spawn_local(async move {
                autoclone!(terminal_id);
                if let Err(error) =
                    api::client::set_title::set_title(&terminal_id, title.to_string()).await
                {
                    warn!("Failed to update title: {error}")
                }
            });
        });
        Self(Rc::new(TerminalTabInner {
            id: terminal_id,
            title,
            selected,
            xtermjs: Mutex::new(None),
            registrations,
        }))
    }
}

impl TabDescriptor for TerminalTab {
    type State = TerminalsState;

    fn key(&self) -> XString {
        self.id.clone().into()
    }

    #[autoclone]
    #[html]
    fn title(&self, state: &TerminalsState) -> impl Into<XNode> {
        let id = &self.id;
        let title = &self.title;
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
                autoclone!(id);
                ev.stop_propagation();
                wasm_bindgen_futures::spawn_local(async move {
                    autoclone!(id);
                    api::client::stream::try_restart_pipe();
                    api::client::stream::close(id, None).await;
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
                    widgets::link::link(
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
            .field(&self.id.as_str())
            .finish()
    }
}

impl PartialEq for TerminalTabInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TerminalTabInner {}
