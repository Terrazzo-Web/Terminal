#![cfg(feature = "client")]

use std::path::Path;
use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use web_sys::MouseEvent;

use self::diagnostics::debug;
use super::fsio::FileMetadata;
use super::manager::EditorState;
use super::manager::TextEditorManager;
use super::notify::EventKind;
use super::notify::FileEventKind;
use crate::frontend::menu::before_menu;
use crate::frontend::timestamp;
use crate::frontend::timestamp::datetime::DateTime;
use crate::frontend::timestamp::display_timestamp;
use crate::utils::more_path::MorePath as _;

stylance::import_crate_style!(style, "src/text_editor/folder.scss");

#[autoclone]
#[html]
#[template(tag = div)]
pub fn folder(
    manager: Ptr<TextEditorManager>,
    editor_state: EditorState,
    list: Arc<Vec<FileMetadata>>,
) -> XElement {
    let path = &editor_state.path;
    let file_path = &path.file;

    let mut rows = vec![];
    let parent_path = Path::new(file_path.as_ref()).parent();
    let parent = parent_path.map(|_| FileMetadata {
        name: "..".into(),
        is_dir: true,
        ..FileMetadata::default()
    });

    let notify_registration =
        manager
            .notify_service
            .watch_folder(&editor_state.path, move |event| {
                autoclone!(path, manager);
                debug!("Folder view notification: {event:?}");
                let EventKind::File(kind) = event.kind else {
                    return;
                };
                match (Path::new(&event.path) == path.as_deref().full_path(), kind) {
                    (
                        false,
                        FileEventKind::Create | FileEventKind::Modify | FileEventKind::Delete,
                    ) => {
                        debug!("File inside the folder was added/removed ==> refresh the view");
                    }
                    (true, FileEventKind::Create) => {
                        debug!("The folder was created !?!");
                        return;
                    }
                    (true, FileEventKind::Modify) => {
                        debug!("The folder was modified");
                    }
                    (true, FileEventKind::Delete) => {
                        debug!("The folder was deleted");
                        manager.path.file.update(|file_path| {
                            let file_path = Path::new(file_path.as_ref());
                            let parent = file_path.parent().unwrap_or_else(|| "/".as_ref());
                            Some(parent.to_owned_string().into())
                        });
                        return;
                    }
                    (true | false, FileEventKind::Error) => {
                        debug!("Error polling notifications");
                        return;
                    }
                }

                debug!("Force reload folder view");
                manager
                    .path
                    .file
                    .force(manager.path.file.get_value_untracked());
            });

    for file in parent.iter().chain(list.iter()) {
        let name = &file.name;
        let is_dir = file.is_dir;
        let display_name = if is_dir {
            format!("{name}/")
        } else {
            name.to_string()
        };
        let size = file.size.map(print_size).unwrap_or_else(|| "-".to_owned());
        let modified = file
            .modified
            .map(DateTime::from_utc)
            .map(|m| timestamp(display_timestamp(m)))
            .unwrap_or_else(|| span("-"));
        let user = file.user.clone().unwrap_or_default();
        let group = file.group.clone().unwrap_or_default();
        let permissions = file
            .mode
            .map(|m| {
                format!(
                    "{}{}",
                    if is_dir { 'd' } else { '-' },
                    mode_to_permissions(m)
                )
            })
            .unwrap_or_default();
        rows.push(tr(
            id = "{name}",
            click = move |_| {
                autoclone!(manager, file_path, name);
                let file_path = &*file_path;
                let file_path = file_path.trim_start_matches('/');
                let file = if &*name == ".." {
                    Path::new(file_path)
                        .parent()
                        .map(Path::to_owned)
                        .unwrap_or_default()
                } else {
                    Path::new(file_path).join(&*name)
                };
                manager.path.file.set(file.to_owned_string())
            },
            td("{display_name}"),
            td("{size}"),
            td(modified),
            td("{user}"),
            td("{group}"),
            td("{permissions}"),
        ));
    }
    tag(
        class = style::folder,
        table(
            thead(tr(
                th("Name"),
                th("Size"),
                th("Modified"),
                th("User"),
                th("Group"),
                th("Permissions"),
            )),
            tbody(
                mouseover = move |_: MouseEvent| {
                    if let Some(f) = before_menu().take() {
                        f()
                    };
                },
                rows..,
            ),
        ),
        after_render = move |_| {
            let _moved = &notify_registration;
        },
    )
}

#[html]
#[template(tag = span)]
fn timestamp(#[signal] mut t: Box<timestamp::Timestamp>) -> XElement {
    tag(
        "{t}",
        before_render = move |_| {
            let _moved = &t_mut;
        },
    )
}

fn mode_to_permissions(mode: u32) -> String {
    // Unix permission bits: user, group, others
    const PERMISSIONS: [(u32, char); 9] = [
        (0o400, 'r'), // user read
        (0o200, 'w'), // user write
        (0o100, 'x'), // user execute
        (0o040, 'r'), // group read
        (0o020, 'w'), // group write
        (0o010, 'x'), // group execute
        (0o004, 'r'), // other read
        (0o002, 'w'), // other write
        (0o001, 'x'), // other execute
    ];

    PERMISSIONS
        .iter()
        .map(|(bit, ch)| if mode & bit != 0 { *ch } else { '-' })
        .collect()
}

fn print_size(size: u64) -> String {
    if size < 1000 {
        return format!("{size}b");
    }

    let mut sizef = size as f64 / 1024.;
    for suffix in ["Kb", "Mb", "Gb", "Tb"] {
        if sizef < 1000. {
            return format!("{sizef:.2}{suffix}");
        }
        sizef /= 1024.
    }
    return format!("{sizef:.2}Pb");
}

#[cfg(test)]
mod tests {
    #[test]
    fn print_size() {
        assert_eq!(
            "[123b, 120.24Kb, 117.42Mb, 114.67Gb, 111.98Tb, 109.36Pb]",
            format!(
                "[{}, {}, {}, {}, {}, {}]",
                super::print_size(123),
                super::print_size(123123),
                super::print_size(123123123),
                super::print_size(123123123123),
                super::print_size(123123123123123),
                super::print_size(123123123123123123)
            )
        )
    }
}
