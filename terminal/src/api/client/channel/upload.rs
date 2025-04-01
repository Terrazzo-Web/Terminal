use futures::Sink;
use futures::Stream;
use serde::Serialize;
use web_sys::RequestInit;
use web_sys::js_sys;

use crate::api::client::request::Method;
use crate::api::client::request::send_request;

#[allow(unused)]
pub async fn into_upload_stream<I: Serialize>(
    url: &str,
    on_request: impl FnOnce(&RequestInit),
    stream: impl Stream<Item = I>,
) -> (String, impl Sink<I, Error = std::io::Error>) {
    let correlation_id = format!("X{}", js_sys::Math::random());
    let response = send_request(Method::POST, url, on_request).await;
    (
        correlation_id,
        futures::sink::unfold(0, async |s, i| {
            let _ = i;
            Ok(s + 1)
        }),
    )
}
