#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast;
use web_sys::HtmlOptionElement;
use web_sys::HtmlSelectElement;

use crate::api::client_address::ClientAddress;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;
use crate::portforward::manager::Manager;
use crate::portforward::schema::HostPortDefinition;
use crate::portforward::schema::PortForward;

stylance::import_crate_style!(style, "src/portforward/port_forward.scss");

/// The UI for the port forward app.
#[html]
#[template]
pub fn port_forward() -> XElement {
    let manager = Manager::new();
    div(class = style::outer, port_forward_impl(manager))
}

#[html]
fn port_forward_impl(manager: Manager) -> XElement {
    let remote = manager.remote();
    let port_forwards = manager.port_forwards();
    div(
        class = style::inner,
        key = "port-forward",
        div(class = style::header, menu(), show_remote(remote.clone())),
        show_port_forwards(manager, remote, port_forwards),
    )
}

#[autoclone]
#[html]
#[template(tag = div)]
fn show_port_forwards(
    manager: Manager,
    #[signal] remote: Remote,
    #[signal] port_forwards: Arc<[PortForward]>,
) -> XElement {
    manager.load_port_forwards(remote.clone());
    let port_forward_tags = port_forwards
        .iter()
        .map(|port_forward| show_port_forward(&manager, &remote, port_forward));
    tag(
        class = style::port_forwards,
        port_forward_tags..,
        div(
            "+",
            style::cursor = "pointer",
            click = move |_| {
                autoclone!(remote);
                manager.update(&remote, |port_forwards| {
                    let port_forwards = port_forwards.iter().cloned();
                    let port_forwards = port_forwards.chain(Some(PortForward::new()));
                    port_forwards.collect::<Vec<_>>().into()
                });
            },
        ),
    )
}

#[html]
fn show_port_forward(manager: &Manager, remote: &Remote, port_forward: &PortForward) -> XElement {
    let title = port_forward.to_string();
    let PortForward { id, from, to } = port_forward;
    div(
        class = style::port_forward,
        div(class = style::title, "{title}"),
        div(
            class = style::port_forward_body,
            div(
                class = style::from,
                host_port_definition(manager, remote, "From", *id, from, |old, new| {
                    Some(PortForward {
                        id: old.id,
                        from: new,
                        to: old.to.clone(),
                    })
                }),
            ),
            div(
                class = style::to,
                host_port_definition(manager, remote, "To", *id, to, |old, new| {
                    Some(PortForward {
                        id: old.id,
                        from: old.from.clone(),
                        to: new,
                    })
                }),
            ),
        ),
    )
}

#[autoclone]
#[html]
fn host_port_definition(
    manager: &Manager,
    remote: &Remote,
    endpoint: &'static str,
    id: i32,
    host_port_definition: &HostPortDefinition,
    set: impl FnOnce(&PortForward, HostPortDefinition) -> Option<PortForward> + Clone + 'static,
) -> XElement {
    let HostPortDefinition {
        remote: selected_remote,
        host,
        port,
    } = host_port_definition.clone();
    let remote = remote.clone();
    let host = host.clone();
    div(
        class = style::host_port_definition,
        div(class = style::endpoint, "{endpoint}"),
        div(
            class = style::remote,
            label(r#for = format!("remote-{id}"), "Remote: "),
            show_remote_select(
                format!("host-{id}"),
                manager.remotes(),
                selected_remote.clone(),
                move |new_selected_remote| {
                    autoclone!(manager, remote, host, set);
                    manager.set(&remote, id, move |port_forward| {
                        autoclone!(host, new_selected_remote, set);
                        let new = HostPortDefinition {
                            remote: new_selected_remote.clone(),
                            host: host.clone(),
                            port,
                        };
                        set(port_forward, new)
                    });
                },
            ),
        ),
        div(
            class = style::host,
            label(r#for = format!("host-{id}"), "Host: "),
            input(
                r#type = "text",
                id = format!("host-{id}"),
                value = host.to_owned(),
                change = move |_| {
                    autoclone!(manager, set, host, selected_remote, remote);
                    manager.set(&remote, id, |port_forward| {
                        autoclone!(set);
                        let new = HostPortDefinition {
                            remote: selected_remote.clone(),
                            host: host.clone(),
                            port,
                        };
                        set(port_forward, new)
                    })
                },
            ),
        ),
        div(
            class = style::port,
            label(r#for = format!("port-{id}"), "Port: "),
            input(
                r#type = "text",
                id = format!("port-{id}"),
                value = host_port_definition.port.to_string(),
                change = move |_| {
                    autoclone!(manager, remote, host, set);
                    manager.set(&remote, id, |port_forward| {
                        autoclone!(set);
                        let new = HostPortDefinition {
                            remote: selected_remote.clone(),
                            host: host.clone(),
                            port,
                        };
                        set(port_forward, new)
                    })
                },
            ),
        ),
    )
}

#[html]
#[template(tag = select)]
fn show_remote_select(
    tag_id: String,
    #[signal] remotes: Vec<ClientAddress>,
    selected: Remote,
    set: impl Fn(Remote) + Clone + 'static,
) -> XElement {
    let mut options = vec![];
    static LOCAL: &str = "Local";
    let mut selected_index = 0;
    options.push(option(value = "", "{LOCAL}"));
    for (i, remote) in remotes.iter().enumerate() {
        if Some(remote) == selected.as_ref() {
            selected_index = options.len(); // Local is index 0
        }
        options.push(option(value = i.to_string(), "{remote}"))
    }
    if let Some(selected) = &selected {
        if selected_index == 0 {
            // selected_index is "Local" but non-Local remote is selected
            selected_index = options.len();
            options.push(option(
                value = format!("{selected} (offline)"),
                "{selected} (offline)",
                after_render = |option| {
                    let option: HtmlOptionElement = option.dyn_into().or_throw("option");
                    option.set_disabled(true);
                },
            ));
        }
    }
    tag(
        id = tag_id,
        change = move |ev: web_sys::Event| {
            let select = ev.target().or_throw("remote target");
            let select: web_sys::HtmlSelectElement = select.dyn_into().or_throw("remote select");
            let value = select.value();
            if value.is_empty() {
                set(None);
            } else {
                let value: usize = value.parse().or_throw("remote index");
                set(Some(remotes[value].clone()));
            }
        },
        after_render = move |select| {
            let select: HtmlSelectElement = select.dyn_into().or_throw("select");
            select.set_selected_index(selected_index as i32);
        },
        options..,
    )
}

impl std::fmt::Display for PortForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Listen to traffic from {} and forward it to {}",
            self.from, self.to
        )
    }
}

impl std::fmt::Display for HostPortDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}:{}",
            self.remote
                .as_ref()
                .map(|r| r.to_string())
                .unwrap_or_else(|| "Local".to_string()),
            self.host,
            self.port
        )
    }
}
