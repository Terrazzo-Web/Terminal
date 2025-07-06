#![cfg(feature = "server")]

use std::collections::HashMap;
use std::collections::hash_map;
use std::path::Path;
use std::path::PathBuf;

use notify::EventHandler;
use notify::RecommendedWatcher;
use notify::Result;
use notify::Watcher as _;

use crate::text_editor::file_path::FilePath;

pub struct ExtendedWatcher {
    inotify: RecommendedWatcher,
    cargo_workspaces: HashMap<PathBuf, usize>,
}

impl ExtendedWatcher {
    pub fn new<F>(event_handler: F) -> Result<Self>
    where
        F: EventHandler,
    {
        Ok(Self {
            inotify: notify::recommended_watcher(event_handler)?,
            cargo_workspaces: HashMap::default(),
        })
    }

    pub fn watch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        {
            let base = path.base.as_ref();
            if base.exists() && base.join("Cargo.toml").exists() {
                match self.cargo_workspaces.entry(base.to_owned()) {
                    hash_map::Entry::Occupied(mut entry) => {
                        *entry.get_mut() += 1;
                    }
                    hash_map::Entry::Vacant(entry) => {
                        entry.insert(1);
                    }
                }
            }
        }
        self.inotify
            .watch(&path.full_path(), notify::RecursiveMode::NonRecursive)
    }

    pub fn unwatch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        if let hash_map::Entry::Occupied(mut entry) =
            self.cargo_workspaces.entry(path.base.as_ref().to_owned())
        {
            if *entry.get() == 1 {
                entry.remove();
            } else {
                *entry.get_mut() -= 1;
            }
        }
        self.inotify.unwatch(&path.full_path())
    }

    #[expect(unused)]
    pub fn enrich_cargo_workspace(
        &self,
        mut event_handler: impl EventHandler + 'static,
    ) -> impl EventHandler {
        let cargo_workspaces = self.cargo_workspaces.clone();
        move |event: notify::Result<notify::Event>| {
            if let Ok(event) = &event {
                if cargo_workspaces.keys().any(|cargo_workspace| {
                    let mut paths = event.paths.iter();
                    paths.any(|path| path.starts_with(cargo_workspace))
                }) {
                    // TODO process cargo check
                }
            }
            event_handler.handle_event(event)
        }
    }
}
