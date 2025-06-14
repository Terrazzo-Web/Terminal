#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::text_editor::side::SideViewList;
use crate::text_editor::side::SideViewNode;
use crate::text_editor::ui::TextEditor;

#[html]
#[template(tag = div)]
pub fn show_side_view(
    text_editor: Arc<TextEditor>,
    #[signal] side_view: Arc<SideViewList>,
) -> XElement {
    tag(side_view
        .values()
        .map(|child| li(show_side_view_node(&text_editor, child.clone())))
        .collect::<Vec<_>>()..)
}

#[html]
fn show_side_view_node(text_editor: &Arc<TextEditor>, side_view: Arc<SideViewNode>) -> XElement {
    match &*side_view {
        SideViewNode::Folder { name, children } => div(
            key = "folder",
            span("{name}"),
            show_side_view_list(text_editor, children.clone()),
        ),
        SideViewNode::File(file_metadata) => {
            let name = &file_metadata.name;
            div(key = "file", div("{name}"))
        }
    }
}

#[html]
fn show_side_view_list(text_editor: &Arc<TextEditor>, side_view: Arc<SideViewList>) -> XElement {
    ul(side_view
        .values()
        .map(|child| li(show_side_view_node(text_editor, child.clone())))
        .collect::<Vec<_>>()..)
}
