use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::prelude::Ptr;
use terrazzo::prelude::diagnostics;
use terrazzo::prelude::diagnostics::Instrument as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::Response;

use self::diagnostics::debug;
use self::diagnostics::warn;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;
use crate::api::client::terminal_api::BASE_TERMINAL_URL;
use crate::api::shared::terminal_schema::AckRequest;
use crate::api::shared::terminal_schema::STREAMING_WINDOW_SIZE;
use crate::api::shared::terminal_schema::TerminalAddress;

pub fn setup_acks(
    terminal: TerminalAddress,
    stream_reader: impl Stream<Item = Option<Vec<u8>>>,
) -> impl Stream<Item = Option<Vec<u8>>> {
    let terminal = Ptr::new(terminal);
    let unack = 0;
    let prev_ack = {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(()));
        rx
    };
    let stream_reader = stream_reader.scan((unack, prev_ack), move |(unack, prev_ack), chunk| {
        let state = incr_unack(unack, prev_ack, &chunk).map(|state| (state, terminal.clone()));
        return async move {
            if let Some(((ack, prev_ack, new_ack), terminal)) = state {
                match prev_ack.await {
                    Ok(Ok(())) => (),
                    Ok(Err(_ack_error)) => return None,
                    Err(oneshot::Canceled) => {
                        warn!("The previous ack was dropped");
                        return None;
                    }
                }
                let task = async move {
                    let sent_ack = send_ack(&terminal, ack)
                        .await
                        .inspect_err(|error| warn!("Failed to ack: {error}"));
                    let _new_ack = new_ack.send(sent_ack);
                };
                spawn_local(task.in_current_span());
            }
            Some(chunk)
        };
    });
    return Box::pin(stream_reader);
}

fn incr_unack(
    unack: &mut usize,
    prev_ack: &mut oneshot::Receiver<Result<(), AckError>>,
    chunk: &Option<Vec<u8>>,
) -> Option<(
    usize,
    oneshot::Receiver<Result<(), AckError>>,
    oneshot::Sender<Result<(), AckError>>,
)> {
    *unack += chunk.as_ref()?.len();
    if *unack < STREAMING_WINDOW_SIZE / 2 {
        return None;
    }
    let (tx, rx) = oneshot::channel();
    let prev_ack = std::mem::replace(prev_ack, rx);
    let ack = *unack;
    *unack = 0;
    return Some((ack, prev_ack, tx));
}

async fn send_ack(terminal: &TerminalAddress, ack: usize) -> Result<(), AckError> {
    debug!("Send ack={ack}");
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_TERMINAL_URL}/stream/ack"),
        set_json_body(&AckRequest { terminal, ack })?,
    )
    .await?;
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
enum AckError {
    #[error("[{n}] {0}", n = self.name())]
    Body(#[from] serde_json::Error),

    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),
}
