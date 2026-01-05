#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::assets::icons;
use crate::text_editor::manager::TextEditorManager;
use crate::text_editor::side::SideViewList;
use crate::text_editor::side::SideViewNode;
use crate::utils::more_path::MorePath as _;

stylance::import_style!(style, "side.scss");

#[html]
#[template(tag = div, key = "side-view")]
pub fn show_side_view(
    manager: Ptr<TextEditorManager>,
    #[signal] side_view: Arc<SideViewList>,
) -> XElement {
    tag(
        class = style::side,
        show_side_view_list(&manager, "".as_ref(), side_view),
    )
}

#[html]
fn show_side_view_list(
    manager: &Ptr<TextEditorManager>,
    path: &Path,
    side_view: Arc<SideViewList>,
) -> XElement {
    ul(side_view
        .iter()
        .map(|(name, child)| show_side_view_node(manager, path, name, child))
        .collect::<Vec<_>>()..)
}

#[autoclone]
#[html]
fn show_side_view_node(
    manager: &Ptr<TextEditorManager>,
    path: &Path,
    name: &Arc<str>,
    side_view: &Arc<SideViewNode>,
) -> XElement {
    let path: Arc<Path> = Arc::from(path.join(name.as_ref()));
    li(match &**side_view {
        SideViewNode::Folder(children) => {
            let file_path_signal = manager.path.file.clone();
            div(
                key = "folder",
                div(
                    class = style::folder,
                    img(src = icons::folder(), class = style::icon),
                    div(
                        class %= selected_item(manager.path.file.clone(), path.clone()),
                        span(
                            "{name}",
                            click = move |_| {
                                autoclone!(path);
                                file_path_signal.set(path.to_owned_string())
                            },
                        ),
                    ),
                    close_icon(manager, &path),
                ),
                div(
                    class = style::sub_folder,
                    show_side_view_list(manager, &path, children.clone()),
                ),
            )
        }
        SideViewNode::File { metadata, .. } => {
            let name = &metadata.name;
            let file_path_signal = manager.path.file.clone();
            div(
                key = "file",
                class = style::file,
                img(src = icons::file(), class = style::icon),
                div(
                    class %= selected_item(manager.path.file.clone(), path.clone()),
                    span("{name}"),
                    click = move |_| {
                        autoclone!(path);
                        file_path_signal.set(path.to_owned_string())
                    },
                ),
                close_icon(manager, &path),
            )
        }
    })
}

#[template(wrap = true)]
fn selected_item(#[signal] file_path: Arc<str>, path: Arc<Path>) -> XAttributeValue {
    let file_path: &Path = (*file_path).as_ref();
    if file_path == path.as_ref() {
        style::selected_label
    } else {
        style::label
    }
}

#[autoclone]
#[html]
fn close_icon(manager: &Ptr<TextEditorManager>, path: &Arc<Path>) -> XElement {
    img(
        src = icons::close_tab(),
        class = format!("{} {}", style::icon, style::close),
        click = move |_ev| {
            autoclone!(manager, path);
            manager.remove_from_side_view(&path);
        },
    )
}
