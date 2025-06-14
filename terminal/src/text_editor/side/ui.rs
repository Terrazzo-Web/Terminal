#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::text_editor::side::SideView;

#[html]
#[template(tag = div)]
fn show_side_view(side_view: Arc<SideView>) -> XElement {
    match &*side_view {
        SideView::Folder { name, children } => tag(
            key = "folder",
            div("{name}"),
            children
                .values()
                .map(show_side_view_rec)
                .collect::<Vec<_>>()..,
        ),
        SideView::File(file_metadata) => {
            let name = &file_metadata.name;
            tag(key = "file", div("{name}"))
        }
    }
}

fn show_side_view_rec(side_view: &Arc<SideView>) -> XElement {
    show_side_view(side_view.clone())
}
