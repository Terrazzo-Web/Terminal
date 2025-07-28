#![cfg(feature = "server")]

use std::collections::HashMap;

use super::schema::PortForward;

pub fn process(old: &[PortForward], new: &[PortForward]) {
    let old = old
        .iter()
        .map(|old| (old.id, old))
        .collect::<HashMap<_, _>>();
    for new in new {
        process_port_forward(old.get(&new.id).copied(), new);
    }
}

fn process_port_forward(_old: Option<&PortForward>, _new: &PortForward) {
    todo!()
}
