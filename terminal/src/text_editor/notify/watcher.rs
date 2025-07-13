#![cfg(feature = "server")]

use std::collections::HashMap;
use std::collections::hash_map;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use notify::EventHandler;
use notify::RecommendedWatcher;
use notify::Watcher as _;
use server_fn::ServerFnError;
use terrazzo::autoclone;
use tokio::sync::mpsc;
use tracing::Instrument;
use tracing::Span;
use tracing::debug;
use tracing::warn;

use crate::backend::client_service::grpc_error::GrpcError;
use crate::text_editor::file_path::FilePath;
use crate::text_editor::notify::EventKind;
use crate::text_editor::notify::NotifyResponse;
use crate::text_editor::rust_lang::service::CargoCheckError;
use crate::text_editor::rust_lang::service::cargo_check;
use crate::text_editor::rust_lang::synthetic::SyntheticDiagnostic;
use crate::utils::async_throttle::Throttle;
use crate::utils::more_path::MorePath;

pub struct ExtendedWatcher {
    inotify: RecommendedWatcher,
    cargo_workspaces: CargoWorkspaces,
}

#[derive(Clone, Default)]
struct CargoWorkspaces {
    map: Arc<Mutex<HashMap<Arc<Path>, CargoWorkspace>>>,
}

struct CargoWorkspace {
    count: usize,
    run_cargo_check: RunCargoCheck,
}

type RunCargoCheck = Arc<Throttle<Box<dyn Fn(()) -> CargoCheckFuture + Send + Sync>>>;
type CargoCheckFuture = Pin<Box<dyn Future<Output = Option<CargoCheckResult>> + Send + Sync>>;
type CargoCheckResult = Result<Vec<SyntheticDiagnostic>, GrpcError<CargoCheckError>>;
type EventSender = mpsc::UnboundedSender<Result<NotifyResponse, ServerFnError>>;

impl ExtendedWatcher {
    pub fn new<F, H>(tx: EventSender, make_event_handler: F) -> notify::Result<Self>
    where
        F: Fn(EventSender) -> H,
        H: EventHandler,
    {
        let cargo_workspaces = CargoWorkspaces::default();
        Ok(Self {
            inotify: notify::recommended_watcher(
                cargo_workspaces.enrich_cargo_workspace(tx.clone(), make_event_handler(tx)),
            )?,
            cargo_workspaces,
        })
    }

    pub fn watch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        {
            let base = path.base.as_ref();
            if base.exists() && base.join("Cargo.toml").exists() {
                let mut cargo_workspaces = self.cargo_workspaces.map.lock().unwrap();
                match cargo_workspaces.entry(base.into()) {
                    hash_map::Entry::Occupied(mut entry) => {
                        entry.get_mut().count += 1;
                    }
                    hash_map::Entry::Vacant(entry) => {
                        debug!(?base, "Add cargo_workspaces to watch");
                        entry.insert(CargoWorkspace {
                            count: 1,
                            run_cargo_check: Throttle::new(make_run_cargo_check(base)).into(),
                        });
                    }
                }
            }
        }

        let full_path = path.full_path();
        debug!("Start watching {full_path:?}");
        self.inotify
            .watch(&full_path, notify::RecursiveMode::NonRecursive)
    }

    pub fn unwatch(
        &mut self,
        path: FilePath<impl AsRef<Path>, impl AsRef<Path>>,
    ) -> notify::Result<()> {
        let base = path.base.as_ref();
        let mut cargo_workspaces = self.cargo_workspaces.map.lock().unwrap();
        if let hash_map::Entry::Occupied(mut entry) = cargo_workspaces.entry(base.into()) {
            if entry.get().count == 1 {
                debug!(?base, "Remove cargo_workspaces from watch");
                entry.remove();
            } else {
                entry.get_mut().count -= 1;
            }
        }
        let full_path = path.full_path();
        debug!("Stop watching {full_path:?}");
        self.inotify.unwatch(&full_path)
    }
}

fn make_run_cargo_check(dir: &Path) -> Box<dyn Fn(()) -> CargoCheckFuture + Send + Sync> {
    let dir: Arc<Path> = Arc::from(dir);
    Box::new(move |()| {
        let dir = dir.clone();
        Box::pin(async move { Some(cargo_check(&dir, &[]).await) })
    })
}

impl CargoWorkspaces {
    #[autoclone]
    fn enrich_cargo_workspace(
        &self,
        tx: EventSender,
        mut event_handler: impl EventHandler + 'static,
    ) -> impl EventHandler {
        let this = self.clone();
        let runtime = tokio::runtime::Handle::current();
        let span = Span::current();
        debug!("Adding cargo check to EventHandler");
        move |event: notify::Result<notify::Event>| {
            if let Ok(event) = &event {
                match event.kind {
                    notify::EventKind::Create { .. }
                    | notify::EventKind::Modify { .. }
                    | notify::EventKind::Remove { .. } => {}
                    _ => {
                        return;
                    }
                }
                for (cargo_path, run_cargo_check) in this.matches_cargo_workspace(event) {
                    let cargo_check_task = async move {
                        autoclone!(tx);
                        let Some(result) = run_cargo_check.run(()).await else {
                            return;
                        };
                        let diagnostics = match result {
                            Ok(diagnostics) => diagnostics,
                            Err(error) => return warn!("Cargo check failed with: {error}"),
                        };
                        let _ = tx.send(Ok(NotifyResponse {
                            path: cargo_path.to_owned_string(),
                            kind: EventKind::CargoCheck(diagnostics.into()),
                        }));
                    };
                    runtime.spawn(cargo_check_task.instrument(span.clone()));
                }
            }
            event_handler.handle_event(event)
        }
    }

    fn matches_cargo_workspace(&self, event: &notify::Event) -> Vec<(Arc<Path>, RunCargoCheck)> {
        let cargo_workspaces = self.map.lock().unwrap();
        cargo_workspaces
            .iter()
            .filter_map(|(cargo_path, cargo_workspace)| {
                let mut event_paths = event.paths.iter();
                event_paths
                    .any(|event_path| event_path.starts_with(cargo_path))
                    .then(|| (cargo_path.clone(), cargo_workspace.run_cargo_check.clone()))
            })
            .collect()
    }
}
