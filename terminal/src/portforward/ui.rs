#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::declare_trait_aliias;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use web_sys::HtmlOptionElement;
use web_sys::HtmlSelectElement;

use super::manager::Manager;
use super::schema::HostPortDefinition;
use super::schema::HostPortDefinitionImpl;
use super::schema::PortForward;
use super::schema::PortForwardStatus;
use super::sync_state::Fields;
use super::sync_state::SyncState;
use crate::api::client_address::ClientAddress;
use crate::assets::icons;
use crate::frontend::menu::menu;
use crate::frontend::remotes::Remote;
use crate::frontend::remotes_ui::show_remote;
use crate::portforward::schema::PortForwardState;

stylance::import_style!(style, "port_forward.scss");

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

#[html]
#[template(tag = div)]
fn show_port_forwards(
    manager: Manager,
    #[signal] remote: Remote,
    #[signal] port_forwards: Arc<Vec<PortForward>>,
) -> XElement {
    manager.load_port_forwards(remote.clone());
    let port_forward_tags = port_forwards
        .iter()
        .map(|port_forward| show_port_forward(&manager, &remote, port_forward));
    let new_sync_state = XSignal::new("new sync-state", Default::default());
    tag(
        class = style::port_forwards,
        port_forward_tags..,
        div(
            show_add_port_forward(new_sync_state.clone()),
            click = click_add_port_forward(manager, remote, new_sync_state),
        ),
    )
}

fn click_add_port_forward(
    manager: Manager,
    remote: Remote,
    new_sync_state: XSignal<SyncState>,
) -> impl Fn(web_sys::MouseEvent) {
    move |_| {
        manager.update(
            &remote,
            new_sync_state.clone(),
            Fields::all() - Fields::DELETE,
            |port_forwards| {
                let next_id = {
                    let ids = port_forwards.iter().map(|port_forward| port_forward.id);
                    ids.max().unwrap_or(0) + 1
                };
                let port_forwards = port_forwards.iter().cloned();
                let next = PortForward {
                    id: next_id,
                    ..PortForward::default()
                };
                port_forwards.chain(Some(next)).collect::<Vec<_>>().into()
            },
        );
    }
}

#[html]
#[template(tag = div)]
fn show_add_port_forward(#[signal] new_sync_state: SyncState) -> XElement {
    tag(
        class = style::add,
        div(img(src = new_sync_state.add_src())),
        "Add port forward",
    )
}

#[html]
fn show_port_forward(manager: &Manager, remote: &Remote, port_forward: &PortForward) -> XElement {
    let title = port_forward.to_string();
    let PortForward {
        id,
        from,
        to,
        state,
        checked: _,
    } = port_forward;
    let sync_state = XSignal::new("sync-state", SyncState::default());
    let params = ShowHostPortDefinition {
        manager,
        remote,
        sync_state: &sync_state,
        id: *id,
    };
    div(
        class = style::port_forward,
        div(
            class = style::title,
            show_status(sync_state.clone()),
            "{title}",
            show_delete(
                manager.clone(),
                remote.clone(),
                *id,
                sync_state.clone(),
                sync_state.clone(),
            ),
        ),
        div(
            class = style::port_forward_body,
            div(
                class = style::from,
                show_host_port_definition(params, "From", from, |old, new| {
                    Some(PortForward {
                        from: new,
                        ..old.clone()
                    })
                }),
            ),
            div(
                class = style::to,
                show_host_port_definition(params, "To", to, |old, new| {
                    Some(PortForward {
                        to: new,
                        ..old.clone()
                    })
                }),
            ),
        ),
        show_active_checkbox(params, port_forward),
        show_state(state),
    )
}

#[html]
#[autoclone]
fn show_active_checkbox(params: ShowHostPortDefinition, port_forward: &PortForward) -> XElement {
    let ShowHostPortDefinition {
        manager,
        remote,
        sync_state,
        id,
    } = params;
    let checked = port_forward.checked;

    let toggle_status = move |event: web_sys::Event| {
        autoclone!(manager, remote);
        autoclone!(sync_state);
        let target = event.target().or_throw("targtet for toggle_status");
        let target: HtmlInputElement = target.dyn_into().or_throw("input for toggle_status");
        let checked = target.checked();
        manager.set(
            &remote,
            sync_state.clone(),
            Fields::ACTIVE,
            id,
            |port_forward| {
                Some(PortForward {
                    checked,
                    ..port_forward.clone()
                })
            },
        )
    };

    div(
        class = style::active_checkbox,
        label(r#for = format!("active-{id}"), "Active "),
        input(
            r#type = "checkbox",
            id = format!("active-{id}"),
            change = toggle_status,
            checked = checked.then(|| "checked".to_owned()),
        ),
    )
}

#[html]
fn show_state(state: &PortForwardState) -> XElement {
    let state = state.lock();
    let count = state.count;
    match &state.status {
        PortForwardStatus::Pending => div(class = style::port_forward_status, "Pending..."),
        PortForwardStatus::Up => div(class = style::port_forward_status, "Up: {count}"),
        PortForwardStatus::Offline => {
            if count == 0 {
                div(class = style::port_forward_status, "Offline")
            } else {
                div(
                    class = style::port_forward_status,
                    "Pending shutdown: {count}",
                )
            }
        }
        PortForwardStatus::Failed(error) => div(
            class = style::port_forward_status,
            span(style::color = "red", style::font_weight = "bold", "Error: "),
            "{error}",
        ),
    }
}

#[html]
#[template(tag = img)]
fn show_status(#[signal] sync_state: SyncState) -> XElement {
    tag(class = style::status, src = sync_state.status_src())
}

#[autoclone]
#[html]
#[template(tag = img)]
fn show_delete(
    manager: Manager,
    remote: Remote,
    id: i32,
    sync_state_signal: XSignal<SyncState>,
    #[signal] sync_state: SyncState,
) -> XElement {
    tag(
        class = style::delete,
        style::visibility = sync_state.is_deleting().then_some("hidden"),
        src = icons::trash(),
        click = move |_| {
            autoclone!(remote);
            manager.update(
                &remote,
                sync_state_signal.clone(),
                Fields::DELETE,
                |port_forwards| {
                    port_forwards
                        .iter()
                        .filter(|&port_forward| port_forward.id != id)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into()
                },
            );
        },
    )
}

#[derive(Clone, Copy)]
struct ShowHostPortDefinition<'t> {
    manager: &'t Manager,
    remote: &'t Remote,
    sync_state: &'t XSignal<SyncState>,
    id: i32,
}

declare_trait_aliias! {
    EditHostPortDefinition,
    FnOnce(&PortForward, HostPortDefinition) -> Option<PortForward> + Clone + 'static
}

#[autoclone]
#[html]
fn show_host_port_definition(
    params: ShowHostPortDefinition,
    endpoint: &'static str,
    host_port_definition: &HostPortDefinition,
    set: impl EditHostPortDefinition,
) -> XElement {
    let ShowHostPortDefinition {
        manager,
        remote,
        sync_state,
        id,
    } = params;
    let HostPortDefinitionImpl {
        forwarded_remote,
        host,
        port,
    } = &**host_port_definition;
    let port = *port;

    let set_remote = move |forwarded_remote| {
        autoclone!(manager, remote);
        autoclone!(host, set, sync_state);
        manager.set(
            &remote,
            sync_state.clone(),
            Fields::REMOTE,
            id,
            move |port_forward| {
                autoclone!(host, set);
                let new = HostPortDefinition::new(forwarded_remote, host.clone(), port);
                set(port_forward, new)
            },
        );
    };

    let set_host = move |event: web_sys::Event| {
        autoclone!(manager, remote);
        autoclone!(forwarded_remote, set, sync_state);
        let target = event.target().or_throw("targtet for set_host");
        let target: HtmlInputElement = target.dyn_into().or_throw("input for set_host");
        manager.set(
            &remote,
            sync_state.clone(),
            Fields::HOST,
            id,
            |port_forward| {
                autoclone!(forwarded_remote, set);
                let new = HostPortDefinition::new(
                    forwarded_remote,
                    target.value().trim().to_owned(),
                    port,
                );
                set(port_forward, new)
            },
        )
    };

    let set_port = move |event: web_sys::Event| {
        autoclone!(manager, remote);
        autoclone!(forwarded_remote, host, set, sync_state);
        let target = event.target().or_throw("targtet for set_port");
        let target: HtmlInputElement = target.dyn_into().or_throw("input for set_port");
        let port = target.value();
        let Ok(port) = port.trim().parse() else {
            diagnostics::warn!("Value doesn't parse as u16: '{port}'");
            return;
        };
        manager.set(
            &remote,
            sync_state.clone(),
            Fields::PORT,
            id,
            |port_forward| {
                autoclone!(forwarded_remote, host, set);
                let new = HostPortDefinition::new(forwarded_remote, host, port);
                set(port_forward, new)
            },
        )
    };

    div(
        class = style::host_port_definition,
        div(class = style::endpoint, "{endpoint}"),
        div(
            class = style::remote,
            label(r#for = format!("remote-{id}"), "Remote: "),
            show_remote_select(
                format!("host-{id}"),
                manager.remotes(),
                forwarded_remote.clone(),
                set_remote,
            ),
        ),
        div(
            class = style::host,
            label(r#for = format!("host-{id}"), "Host: "),
            input(
                r#type = "text",
                id = format!("host-{id}"),
                value = host.to_owned(),
                change = set_host,
                keydown = move |_event| {
                    autoclone!(sync_state);
                    SyncState::incr_pending(sync_state.clone(), Fields::HOST)
                },
                blur = move |_event| {
                    autoclone!(sync_state);
                    SyncState::decr_pending(sync_state.clone(), Fields::HOST)
                },
                autocomplete = "off",
            ),
        ),
        div(
            class = style::port,
            label(r#for = format!("port-{id}"), "Port: "),
            input(
                r#type = "number",
                min = "1",
                max = "65535",
                id = format!("port-{id}"),
                value = host_port_definition.port.to_string(),
                change = set_port,
                keydown = move |_event| {
                    autoclone!(sync_state);
                    SyncState::incr_pending(sync_state.clone(), Fields::PORT)
                },
                blur = move |_event| {
                    autoclone!(sync_state);
                    SyncState::decr_pending(sync_state.clone(), Fields::PORT)
                },
                autocomplete = "off",
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
