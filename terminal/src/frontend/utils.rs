use std::time::Duration;

use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::prelude::Closure;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;

pub async fn sleep(timeout: Duration) -> Result<(), SleepError> {
    let (tx, rx) = oneshot::channel();
    let closure = Closure::once(|| {
        let _ = tx.send(());
    });
    let _handle: i32 = web_sys::window()
        .expect("window")
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            timeout.as_millis() as i32,
        )
        .map_err(SleepError::SetTimeout)?;
    let () = rx.await.map_err(SleepError::Canceled)?;
    drop(closure);
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SleepError {
    #[error("[{n}] {0:?}", n = self.name())]
    SetTimeout(JsValue),

    #[error("[{n}] {0:?}", n = self.name())]
    Canceled(oneshot::Canceled),
}
