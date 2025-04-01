use std::rc::Rc;

use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use serde::Serialize;
use tracing::Instrument;
use tracing::info_span;
use wasm_bindgen_futures::spawn_local;
use wasm_streams::ReadableStream;
use web_sys::RequestInit;
use web_sys::Response;
use web_sys::js_sys;
use web_sys::js_sys::Uint8Array;

use crate::api::NEWLINE;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::ThenRequest as _;
use crate::api::client::request::send_request;

#[allow(unused)]
pub fn into_upload_stream<O: Serialize>(
    url: &str,
    on_request: impl FnOnce(&RequestInit) + 'static,
    upload: impl Stream<Item = O> + 'static,
    shutdown: oneshot::Sender<Result<Response, Rc<SendRequestError>>>,
) -> String {
    let correlation_id = format!("X{}", js_sys::Math::random());
    let url = url.to_owned();
    let upload_task = async move {
        let upload_task = send_request(
            Method::POST,
            &url,
            set_request_body(upload).then(on_request),
        );
        let _ = shutdown.send(upload_task.await.map_err(Rc::new));
    };
    let () = spawn_local(upload_task.instrument(info_span!("Upload", correlation_id)));
    return correlation_id;
}

fn set_request_body<O: Serialize>(
    upload: impl Stream<Item = O> + 'static,
) -> impl FnOnce(&RequestInit) {
    let stream = into_request_stream(upload);
    move |request| request.set_body(&stream.into_raw())
}

fn into_request_stream<O: Serialize>(stream: impl Stream<Item = O> + 'static) -> ReadableStream {
    let stream = stream.map(|item| {
        serde_json::to_vec(&item)
            .map(|mut chunk| {
                chunk.push(NEWLINE);
                let buffer = Uint8Array::new_with_length(chunk.len() as u32);
                buffer.copy_from(&chunk);
                return chunk.into();
            })
            .map_err(|error| error.to_string().into())
    });
    ReadableStream::from_stream(stream)
}
