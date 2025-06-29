#![cfg(feature = "client")]

use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use server_fn::ServerFnError;
use terrazzo::prelude::diagnostics;
use terrazzo::widgets::debounce::DoDebounce;

use self::diagnostics::warn;
use crate::frontend::remotes::Remote;

pub async fn load_file(
    remote: Remote,
    base_path: Arc<str>,
    file_path: Arc<str>,
) -> Result<Option<super::File>, ServerFnError> {
    super::load_file(remote, base_path, file_path).await
}

pub async fn store_file<P: Send + 'static>(
    remote: Remote,
    base_path: Arc<str>,
    file_path: Arc<str>,
    content: String,
    pending: P,
) {
    assert!(std::mem::needs_drop::<P>());
    static DEBOUNCED_STORE_FILE_FN: OnceLock<StoreFileFn> = OnceLock::new();
    let debounced_store_file_fn = DEBOUNCED_STORE_FILE_FN.get_or_init(make_debounced_store_file_fn);
    let () =
        debounced_store_file_fn((remote, base_path, file_path, content, Box::new(pending))).await;
}

fn make_debounced_store_file_fn() -> StoreFileFn {
    let debounced = Duration::from_secs(3).async_debounce(
        move |(remote, base_path, file_path, content, pending)| async move {
            let () = super::store_file_impl(remote, base_path, file_path, content)
                .await
                .unwrap_or_else(|error| warn!("Failed to store file: {error}"));
            drop(pending);
        },
    );
    return Box::new(debounced);
}

type StoreFileFn =
    Box<dyn Fn((Remote, Arc<str>, Arc<str>, String, Box<dyn Send>)) -> BoxFuture + Send + Sync>;
type BoxFuture = Pin<Box<dyn Future<Output = ()>>>;
