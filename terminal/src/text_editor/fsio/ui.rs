#![cfg(feature = "client")]

use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;

use server_fn::ServerFnError;
use terrazzo::prelude::diagnostics;
use terrazzo::widgets::debounce::DoDebounce;

use self::diagnostics::warn;
use crate::frontend::remotes_list::Remote;
use crate::text_editor::file_path::FilePath;
use crate::text_editor::ui::STORE_FILE_DEBOUNCE_DELAY;

pub async fn load_file(
    remote: Remote,
    path: FilePath<Arc<str>>,
) -> Result<Option<super::File>, ServerFnError> {
    super::load_file(remote, path).await
}

pub async fn store_file<B: Send + 'static, A: Send + 'static>(
    remote: Remote,
    path: FilePath<Arc<str>>,
    content: String,
    before: B,
    after: A,
) {
    assert!(std::mem::needs_drop::<B>());
    assert!(std::mem::needs_drop::<A>());
    static DEBOUNCED_STORE_FILE_FN: OnceLock<StoreFileFn> = OnceLock::new();
    let debounced_store_file_fn = DEBOUNCED_STORE_FILE_FN.get_or_init(make_debounced_store_file_fn);
    let () = debounced_store_file_fn(StoreFileFnArg {
        remote,
        path,
        content,
        before: Box::new(before),
        after: Box::new(after),
    })
    .await;
}

fn make_debounced_store_file_fn() -> StoreFileFn {
    let debounced = STORE_FILE_DEBOUNCE_DELAY.async_debounce(
        move |StoreFileFnArg {
                  remote,
                  path,
                  content,
                  before,
                  after,
              }| async move {
            drop(before);
            let () = super::store_file_impl(remote, path, content)
                .await
                .unwrap_or_else(|error| warn!("Failed to store file: {error}"));
            drop(after);
        },
    );
    return Box::new(debounced);
}

type StoreFileFn = Box<dyn Fn(StoreFileFnArg) -> BoxFuture + Send + Sync>;
type BoxFuture = Pin<Box<dyn Future<Output = ()>>>;

struct StoreFileFnArg {
    remote: Remote,
    path: FilePath<Arc<str>>,
    content: String,
    before: Box<dyn Send>,
    after: Box<dyn Send>,
}
