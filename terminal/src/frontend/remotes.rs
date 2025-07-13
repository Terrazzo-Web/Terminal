#![cfg(any(feature = "text-editor", feature = "converter"))]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use self::diagnostics::debug;
use crate::frontend::remotes_list::Remote;
use crate::frontend::remotes_list::RemotesState;

stylance::import_crate_style!(style, "src/frontend/remotes.scss");

#[html]
#[template(tag = div)]
pub fn show_remote(#[signal] mut cur_remote: Remote) -> XElement {
    let remotes_state = RemotesState::new();

    let cur_remote_name;
    let cur_remote_name = match &cur_remote {
        Some(cur_remote) => {
            cur_remote_name = cur_remote.to_string();
            &cur_remote_name
        }
        None => "Local",
    };
    tag(
        class = style::remotes,
        div(
            "{cur_remote_name}",
            class = style::show_current,
            mouseenter = remotes_state.mouseenter(),
        ),
        mouseleave = remotes_state.mouseleave(),
        remotes_state.show_remotes_dropdown(
            move |remote| {
                let remote_name = remote
                    .map(|remote_name| format!("{remote_name} ⏎"))
                    .unwrap_or_else(|| "Local".into());
                let remote_class = (cur_remote.as_ref() == remote).then_some(style::current);
                (remote_name, remote_class)
            },
            move |_, new_remote| {
                debug!("Set text editor remote to {new_remote:?}");
                cur_remote_mut.set(new_remote)
            },
        ),
    )
}
