#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlOptionElement;
use web_sys::HtmlSelectElement;

use crate::api::client::remotes_api;
use crate::api::client_address::ClientAddress;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;
use crate::portforward::schema::HostPortDefinition;
use crate::portforward::schema::PortForward;

stylance::import_crate_style!(style, "src/portforward/port_forward.scss");

/// The UI for the port forward app.
#[autoclone]
#[html]
#[template]
pub fn port_forward() -> XElement {
    let port_forwards: XSignal<Arc<[PortForward]>> = XSignal::new("port-forwards", Arc::new([]));
    let remote = XSignal::new("remote", Remote::default());
    let remotes_signal = XSignal::new("remotes", vec![]);
    spawn_local(async move {
        autoclone!(remotes_signal);
        let Ok(remotes) = remotes_api::remotes().await else {
            return;
        };
        remotes_signal.set(remotes);
    });
    div(
        class = style::outer,
        port_forward_impl(remotes_signal, remote, port_forwards),
    )
}

#[html]
fn port_forward_impl(
    remotes: XSignal<Vec<ClientAddress>>,
    remote: XSignal<Remote>,
    port_forwards: XSignal<Arc<[PortForward]>>,
) -> XElement {
    div(
        class = style::inner,
        key = "port-forward",
        div(class = style::header, menu(), show_remote(remote.clone())),
        show_port_forwards(remotes, remote, port_forwards.clone(), port_forwards),
    )
}

#[autoclone]
#[html]
#[template(tag = div)]
fn show_port_forwards(
    remotes: XSignal<Vec<ClientAddress>>,
    #[signal] remote: Remote,
    port_forwards_signal: XSignal<Arc<[PortForward]>>,
    #[signal] mut port_forwards: Arc<[PortForward]>,
) -> XElement {
    spawn_local(async move {
        autoclone!(remote, port_forwards_mut);
        let Ok(port_forwards) = super::state::load_port_forwards(remote).await else {
            return;
        };
        port_forwards_mut.set(port_forwards);
    });

    let port_forward_tags = port_forwards
        .iter()
        .enumerate()
        .map(|(index, port_forward)| {
            let id = port_forwards[index].id;
            let set = move |new: Option<PortForward>| {
                autoclone!(remote, port_forwards_signal);
                if let Some(new) = new {
                    assert!(new.id == id, "PortForward id mismatch {} != {}", new.id, id);
                    save_state_on_update(
                        port_forwards_signal.clone(),
                        set_port_forward(id, new),
                        &remote,
                    );
                } else {
                    save_state_on_update(
                        port_forwards_signal.clone(),
                        remove_port_forward(id),
                        &remote,
                    );
                }
            };
            show_port_forward(remotes.clone(), index, port_forward.clone(), set)
        });
    tag(
        class = style::port_forwards,
        port_forward_tags..,
        div(
            "+",
            style::cursor = "pointer",
            click = move |_| {
                autoclone!(remote);
                save_state_on_update(
                    port_forwards_signal.clone(),
                    |port_forwards| {
                        Some(
                            port_forwards
                                .iter()
                                .cloned()
                                .chain(Some(PortForward::new()))
                                .collect::<Vec<_>>()
                                .into(),
                        )
                    },
                    &remote,
                );
            },
        ),
    )
}

fn set_port_forward(
    id: i32,
    new: PortForward,
) -> impl FnOnce(&Arc<[PortForward]>) -> Option<Arc<[PortForward]>> {
    move |old| {
        let new = old
            .iter()
            .map(|old| {
                if old.id == id {
                    new.clone()
                } else {
                    old.clone()
                }
            })
            .collect::<Vec<_>>();
        Some(new.into())
    }
}

fn remove_port_forward(id: i32) -> impl FnOnce(&Arc<[PortForward]>) -> Option<Arc<[PortForward]>> {
    move |old| {
        let new = old
            .iter()
            .filter(|old| old.id != id)
            .cloned()
            .collect::<Vec<_>>();
        Some(new.into())
    }
}

#[autoclone]
fn save_state_on_update(
    port_forwards: XSignal<Arc<[PortForward]>>,
    update_fn: impl FnOnce(&Arc<[PortForward]>) -> Option<Arc<[PortForward]>>,
    remote: &Remote,
) {
    let current = port_forwards.get_value_untracked();
    let Some(new) = update_fn(&current) else {
        return;
    };
    spawn_local(async move {
        autoclone!(remote);
        let Ok(()) = super::state::store_port_forwards(remote, new.clone())
            .await
            .inspect_err(|error| diagnostics::warn!("Failed to save port forwards: {error}"))
        else {
            return;
        };
        port_forwards.set(new);
    })
}

#[autoclone]
#[html]
fn show_port_forward(
    remotes: XSignal<Vec<ClientAddress>>,
    index: usize,
    port_forward: PortForward,
    set: impl Fn(Option<PortForward>) + Clone + 'static,
) -> XElement {
    let title = port_forward.to_string();
    let PortForward { id, from, to } = port_forward;
    div(
        class = style::port_forward,
        div(class = style::title, "{title}"),
        div(
            class = style::port_forward_body,
            div(
                class = style::from,
                host_port_definition(remotes.clone(), index, from.clone(), move |new| {
                    autoclone!(set, to);
                    set(new.map(|new| PortForward {
                        id,
                        from: new,
                        to: to.clone(),
                    }))
                }),
            ),
            div(
                class = style::to,
                host_port_definition(remotes, index, to, move |new| {
                    autoclone!(set);
                    set(new.map(|new| PortForward {
                        id,
                        from: from.clone(),
                        to: new,
                    }))
                }),
            ),
        ),
    )
}

#[autoclone]
#[html]
fn host_port_definition(
    remotes: XSignal<Vec<ClientAddress>>,
    index: usize,
    host_port_definition: HostPortDefinition,
    set: impl Fn(Option<HostPortDefinition>) + Clone + 'static,
) -> XElement {
    let HostPortDefinition { remote, host, port } = host_port_definition;
    div(
        class = style::host_port_definition,
        div(
            class = style::remote,
            label(r#for = format!("remote-{index}"), "Remote: "),
            show_remote_select(remotes, remote, move |remote| {
                autoclone!(host);
                set(Some(HostPortDefinition {
                    remote,
                    host: host.clone(),
                    port,
                }))
            }),
        ),
        div(
            class = style::host,
            label(r#for = format!("host-{index}"), "Host: "),
            input(r#type = "text", id = format!("host-{index}"), value = host),
        ),
        div(
            class = style::port,
            label(r#for = format!("port-{index}"), "Port: "),
            input(
                r#type = "text",
                id = format!("port-{index}"),
                value = host_port_definition.port.to_string(),
            ),
        ),
    )
}

#[html]
#[template(tag = select)]
fn show_remote_select(
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
