#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::assets::icons;
use crate::text_editor::manager::TextEditor;
use crate::text_editor::side;
use crate::text_editor::side::SideViewList;
use crate::text_editor::side::SideViewNode;

stylance::import_crate_style!(style, "src/text_editor/side.scss");

#[html]
#[template(tag = div, key = "side-view")]
pub fn show_side_view(
    text_editor: Arc<TextEditor>,
    #[signal] side_view: Arc<SideViewList>,
) -> XElement {
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
        .iter()
        .map(|(name, child)| show_side_view_node(text_editor, path, name, child))
        .collect::<Vec<_>>()..)
}

#[autoclone]
#[html]
fn show_side_view_node(
    text_editor: &Arc<TextEditor>,
    path: &Path,
    name: &Arc<str>,
    side_view: &Arc<SideViewNode>,
) -> XElement {
    let path: Arc<Path> = Arc::from(path.join(name.as_ref()));
    li(match &**side_view {
        SideViewNode::Folder(children) => {
            let file_path_signal = text_editor.file_path.clone();
            div(
                key = "folder",
                div(
                    class = style::folder,
                    img(src = icons::folder(), class = style::icon),
                    div(
                        class %= move |t| {
                            autoclone!(text_editor, path);
                            selected_item(t, text_editor.file_path.clone(), path.clone())
                        },
                        span(
                            "{name}",
                            click = move |_| {
                                autoclone!(path);
                                file_path_signal.set(path.to_string_lossy().to_string())
                            },
                        ),
                    ),
                    close_icon(text_editor, &path),
                ),
                div(
                    class = style::sub_folder,
                    show_side_view_list(text_editor, &path, children.clone()),
                ),
            )
        }
        SideViewNode::File(file_metadata) => {
            let name = &file_metadata.name;
            let file_path_signal = text_editor.file_path.clone();
            div(
                key = "file",
                class = style::file,
                img(src = icons::file(), class = style::icon),
                div(
                    class %= move |t| {
                        autoclone!(text_editor, path);
                        selected_item(t, text_editor.file_path.clone(), path.clone())
                    },
                    span("{name}"),
                    click = move |_| {
                        autoclone!(path);
                        file_path_signal.set(path.to_string_lossy().to_string())
                    },
                ),
                close_icon(text_editor, &path),
            )
        }
    })
}

#[template]
pub fn selected_item(#[signal] file_path: Arc<str>, path: Arc<Path>) -> XAttributeValue {
    let file_path: &Path = (*file_path).as_ref();
    if file_path == path.as_ref() {
        style::selected_label
    } else {
        style::label
    }
}

#[autoclone]
#[html]
fn close_icon(text_editor: &Arc<TextEditor>, path: &Arc<Path>) -> XElement {
    img(
        src = icons::close_tab(),
        class = format!("{} {}", style::icon, style::close,),
        click = move |_ev| {
            autoclone!(text_editor, path);
            text_editor.side_view.update(|side_view| {
                let path_vec: Vec<Arc<str>> = path
                    .iter()
                    .map(|leg| leg.to_string_lossy().to_string().into())
                    .collect();
                text_editor.notify_service.unwatch(
                    &text_editor.base_path.get_value_untracked(),
                    &path.to_string_lossy(),
                );
                side::mutation::remove_file(side_view.clone(), &path_vec).ok()
            });
        },
    )
}
