#![cfg(feature = "client")]

use std::sync::Arc;

use chrono::DateTime;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::frontend::timestamp;
use crate::frontend::timestamp::display_timestamp;
use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::ui::EditorState;
use crate::text_editor::ui::TextEditor;

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
        let size = size
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "-".to_owned());
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
                text_editor
                    .file_path
                    .set(Arc::from(format!("{file_path}/{name}")))
            },
            td("{name}"),
            td("{size}"),
            td(modified),
            td("{user}"),
            td("{group}"),
            td("{permissions}"),
        ));
    }
    tag(table(
        thead(tr(
            td("Name"),
            td("Size"),
            td("Modified"),
            td("User"),
            td("Group"),
            td("Permissions"),
        )),
        tbody(rows..),
    ))
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
