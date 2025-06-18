#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::assets::icons;
use crate::text_editor::side::SideViewList;
use crate::text_editor::side::SideViewNode;
use crate::text_editor::ui::TextEditor;

stylance::import_crate_style!(style, "src/text_editor/side.scss");

#[html]
#[template(tag = div)]
pub fn show_side_view(
    text_editor: Arc<TextEditor>,
    #[signal] side_view: Arc<SideViewList>,
) -> XElement {
    let _ = icons::file(); // Referenced in CSS
    let _ = icons::folder(); // Referenced in CSS
    tag(
        class = style::side,
        show_side_view_list(&text_editor, "".as_ref(), side_view),
    )
}

#[html]
fn show_side_view_list(
    text_editor: &Arc<TextEditor>,
    path: &Path,
    side_view: Arc<SideViewList>,
) -> XElement {
    ul(side_view
        .values()
        .map(|child| show_side_view_node(text_editor, path, child.clone()))
        .collect::<Vec<_>>()..)
}

#[autoclone]
#[html]
fn show_side_view_node(
    text_editor: &Arc<TextEditor>,
    path: &Path,
    side_view: Arc<SideViewNode>,
) -> XElement {
    match &*side_view {
        SideViewNode::Folder { name, children } => {
            let path = path.join(name.as_ref());
            let file_path_signal = text_editor.file_path.clone();
            li(
                class = style::folder,
                div(
                    key = "folder",
                    class = style::folder,
                    span(
                        "{name}",
                        click = move |_| {
                            autoclone!(path);
                            file_path_signal.set(path.to_string_lossy().to_string())
                        },
                    ),
                    show_side_view_list(text_editor, &path, children.clone()),
                ),
            )
        }
        SideViewNode::File(file_metadata) => {
            let name = &file_metadata.name;
            let path = path.join(name.as_ref());
            let file_path_signal = text_editor.file_path.clone();
            li(
                class = style::file,
                div(
                    key = "file",
                    class = style::file,
                    span(
                        "{name}",
                        click = move |_| {
                            autoclone!(path);
                            file_path_signal.set(path.to_string_lossy().to_string())
                        },
                    ),
                ),
            )
        }
    }
}
