#![cfg(feature = "client")]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::frontend::remotes::Remote;
use crate::frontend::remotes::RemotesState;
use crate::text_editor::style;

#[html]
#[template(tag = div)]
pub fn show_remote(#[signal] mut remote: Remote) -> XElement {
    let remotes_state = RemotesState::new();

    let remote_name;
    let remote_name = match remote {
        Some(remote) => {
            remote_name = remote.to_string();
            &remote_name
        }
        None => "Local",
    };
    tag(
        class = style::remotes,
        div("{remote_name}", mouseenter = remotes_state.mouseenter()),
        mouseleave = remotes_state.mouseleave(),
        remotes_state.show_remotes_dropdown(move |_, new_remote| remote_mut.set(new_remote)),
    )
}
