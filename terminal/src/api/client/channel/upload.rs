use std::future::ready;
use std::rc::Rc;

use futures::Stream;
use futures::StreamExt as _;
use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Serialize;
use terrazzo::autoclone;
use tracing::Instrument;
use tracing::Span;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_streams::ReadableStream;
use web_sys::RequestInit;
use web_sys::js_sys;
use web_sys::js_sys::Uint8Array;

use crate::api::NEWLINE;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::ThenRequest as _;
use crate::api::client::request::send_request;
use crate::api::client::request::set_correlation_id;
use crate::api::client::request::set_headers;

#[autoclone]
pub fn into_upload_stream<O: Serialize>(
    url: &str,
    on_request: impl FnOnce(&RequestInit) + 'static,
    upload: impl Stream<Item = O> + 'static,
    end_of_upload: oneshot::Sender<UploadError>,
    end_of_download: oneshot::Receiver<()>,
) -> String {
    let correlation_id = format!("X{}", js_sys::Math::random());
    let url = url.to_owned();

    let end_of_upload = {
        let end_of_upload = Rc::new(std::sync::Mutex::new(Some(end_of_upload)));
        let span = Span::current();
        move |error| {
            if let Some(end_of_upload) = end_of_upload.lock().expect("end_of_upload").take() {
                if let Err(error) = end_of_upload.send(error) {
                    let _span = span.enter();
                    warn!("Failed to notify upload failure: {error}");
                }
            }
        }
    };

    let end_of_download = async move {
        match end_of_download.await {
            Ok(()) => info!("Download EOS"),
            Err(oneshot::Canceled) => info!("Download canceled"),
        }
    };

    let upload = upload.take_until(end_of_download);
    let upload_task = async move {
        autoclone!(correlation_id);
        let response = send_request(
            Method::POST,
            &url,
            set_headers(set_correlation_id(correlation_id.as_str()))
                .then(set_request_body(upload, end_of_upload.clone()))
                .then(on_request),
        )
        .await;
        match response.map_err(Rc::new).map_err(UploadError::Request) {
            Ok(response) => info!("Response: {} {}", response.status(), response.status_text()),
            Err(error) => end_of_upload(error),
        }
    };
    let () = spawn_local(upload_task.instrument(info_span!("Upload", correlation_id)));
    return correlation_id;
}

fn set_request_body<O: Serialize>(
    upload: impl Stream<Item = O> + 'static,
    end_of_upload: impl Fn(UploadError) + 'static,
) -> impl FnOnce(&RequestInit) {
    let stream = into_request_stream(upload, end_of_upload);
    move |request| request.set_body(&stream.into_raw())
}

fn into_request_stream<O: Serialize>(
    stream: impl Stream<Item = O> + 'static,
    end_of_upload: impl Fn(UploadError) + 'static,
) -> ReadableStream {
    let stream = stream
        .map(|item| {
            serde_json::to_vec(&item).map(|mut chunk| {
                chunk.push(NEWLINE);
                let buffer = Uint8Array::new_with_length(chunk.len() as u32);
                buffer.copy_from(&chunk);
                return JsValue::from(buffer);
            })
        })
        .map(move |chunk| match chunk {
            Ok(chunk) => Ok(chunk),
            Err(error) => {
                end_of_upload(UploadError::Json(error.into()));
                Err(JsValue::undefined())
            }
        })
        .take_while(|chunk| ready(chunk.is_ok()));
    ReadableStream::from_stream(stream)
}

#[nameth]
#[derive(thiserror::Error, Debug, Clone)]
pub enum UploadError {
    #[error("[{n}] Failed to open JSON stream: {0}", n = self.name())]
    Request(Rc<SendRequestError>),

    #[error("[{n}] {0}", n = self.name())]
    Json(Rc<serde_json::Error>),

    #[error("[{n}] Upload stream canceled", n = self.name())]
    Canceled,
}
