#![cfg(feature = "server")]

use std::path::Path;

use notify::EventHandler;
use notify::RecommendedWatcher;
use notify::Result;
use notify::Watcher as _;

use crate::text_editor::file_path::FilePath;

pub struct ExtendedWatcher {
    inotify: RecommendedWatcher,
}

impl ExtendedWatcher {
    pub fn new<F>(event_handler: F) -> Result<Self>
    where
        F: EventHandler,
    {
        Ok(Self {
            inotify: notify::recommended_watcher(event_handler)?,
        })
    }

    pub fn watch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        self.inotify
            .watch(&path.full_path(), notify::RecursiveMode::NonRecursive)
    }

    pub fn unwatch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        self.inotify.unwatch(&path.full_path())
    }
}
