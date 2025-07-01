#![cfg(feature = "client")]

use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use terrazzo::prelude::*;

use super::fsio;
use super::notify::ui::NotifyService;
use super::side::SideViewList;
use super::synchronized_state::SynchronizedState;
use crate::frontend::remotes::Remote;
use crate::text_editor::fsio::FileMetadata;
use crate::text_editor::side;
use crate::utils::more_path::MorePath as _;

pub(super) struct TextEditorManager {
    pub remote: Remote,
    pub path: FilePath<XSignal<Arc<str>>>,
    pub force_edit_path: XSignal<bool>,
    pub editor_state: XSignal<Option<EditorState>>,
    pub synchronized_state: XSignal<SynchronizedState>,
    pub side_view: XSignal<Arc<SideViewList>>,
    pub notify_service: Arc<NotifyService>,
}

#[derive(Clone)]
pub(super) struct EditorState {
    pub path: FilePath<Arc<str>>,
    pub data: Arc<fsio::File>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FilePath<BASE, FILE = BASE> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    pub base: BASE,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file: FILE,
}

impl TextEditorManager {
    pub fn watch_file(
        &self,
        metadata: &Arc<FileMetadata>,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) {
        let file_path = path.file.as_ref();
        let relative_path = file_path
            .iter()
            .map(|leg| Arc::from(leg.to_owned_string()))
            .collect::<Vec<_>>();
        self.notify_service.watch(path);
        self.side_view.update(|tree| {
            Some(side::mutation::add_file(
                tree.clone(),
                relative_path.as_slice(),
                super::side::SideViewNode::File(metadata.clone()),
            ))
        });
        self.force_edit_path.set(false);
    }

    pub fn unwatch_file(&self, file_path: impl AsRef<Path>) {
        let file_path = file_path.as_ref();
        self.side_view.update(|side_view| {
            self.notify_service.unwatch(FilePath {
                base: self.path.base.get_value_untracked().as_ref(),
                file: file_path,
            });
            let file_path_vec: Vec<Arc<str>> = file_path
                .iter()
                .map(|leg| leg.to_owned_string().into())
                .collect();
            side::mutation::remove_file(side_view.clone(), &file_path_vec).ok()
        });
        self.path.file.update(|old| {
            if Path::new(old.as_ref()) == file_path {
                Some("".into())
            } else {
                None
            }
        })
    }
}

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("path", &self.path)
            .field("data", &self.data)
            .finish()
    }
}

impl<B: AsRef<Path>, F: AsRef<Path>> FilePath<B, F> {
    pub fn full_path(&self) -> PathBuf {
        self.base.as_ref().join(self.file.as_ref())
    }
}

impl<B: Deref, F: Deref> FilePath<B, F> {
    pub fn as_ref(&self) -> FilePath<&B::Target, &F::Target> where {
        FilePath {
            base: &self.base,
            file: &self.file,
        }
    }
}

impl<T> FilePath<T> {
    #[allow(unused)]
    pub fn map<U>(self, f: impl Fn(T) -> U) -> FilePath<U> {
        self.map2(&f, &f)
    }
}

impl<B, F> FilePath<B, F> {
    #[allow(unused)]
    pub fn map2<BB, FF>(
        self,
        b: impl FnOnce(B) -> BB,
        f: impl FnOnce(F) -> FF,
    ) -> FilePath<BB, FF> {
        FilePath {
            base: b(self.base),
            file: f(self.file),
        }
    }
}
