#![cfg(feature = "client")]

use std::path::PathBuf;
use std::sync::Arc;

use chrono::DateTime;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use web_sys::MouseEvent;

use crate::frontend::menu::before_menu;
use crate::frontend::timestamp;
use crate::frontend::timestamp::display_timestamp;
use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::ui::EditorState;
use crate::text_editor::ui::TextEditor;

stylance::import_crate_style!(style, "src/text_editor/folder.scss");

#[autoclone]
#[html]
#[template(tag = div)]
pub fn folder(
    text_editor: Arc<TextEditor>,
    editor_state: EditorState,
    list: Arc<Vec<FileMetadata>>,
) -> XElement {
    let EditorState { file_path, .. } = editor_state;

    let mut rows = vec![];
    for file in list.iter() {
        let FileMetadata {
            name,
            size,
            is_dir,
            created: _,
            accessed: _,
            modified,
            mode,
            user,
            group,
        } = file;
        let name = name.clone();
        let size = size.map(print_size).unwrap_or_else(|| "-".to_owned());
        let modified = modified
            .as_ref()
            .and_then(|m| DateTime::from_timestamp_millis(m.as_millis() as i64))
            .map(|m| timestamp(display_timestamp(m)))
            .unwrap_or_else(|| span("-"));
        let user = user.clone().unwrap_or_default();
        let group = group.clone().unwrap_or_default();
        let permissions = mode
            .map(|m| {
                format!(
                    "{}{}",
                    if *is_dir { 'd' } else { '-' },
                    mode_to_permissions(m)
                )
            })
            .unwrap_or_default();
        rows.push(tr(
            click = move |_| {
                autoclone!(text_editor, file_path);
                let mut file = PathBuf::from(file_path.as_ref());
                file.push(name.as_ref());
                let file = file.to_string_lossy().to_string();
                text_editor.file_path.set(Arc::from(file))
            },
            td("{name}"),
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
    )
}

#[html]
#[template(tag = span)]
fn timestamp(#[signal] mut t: Box<timestamp::Timestamp<chrono::Utc>>) -> XElement {
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
