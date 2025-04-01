#![allow(unused)]

use std::marker::PhantomData;

use futures::Sink;
use futures::Stream;
use json_stream::JsonStreamError;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use web_sys::RequestInit;

use self::json_sink::to_json_sink;
use self::json_stream::to_json_stream;
use super::request::Method;
use super::request::SendRequestError;
use super::request::send_request;

pub mod json_sink;
pub mod json_stream;

pub async fn open_channel<I, O: for<'t> Deserialize<'t>>(
    url: String,
    on_request: impl FnOnce(&RequestInit),
) -> Result<impl WebChannel<Input = I, Output = O>, WebChannelError> {
    let response = send_request(Method::POST, url, on_request)
        .await
        .map_err(WebChannelError::SendRequestError)?;
    let body = response
        .body()
        .ok_or(WebChannelError::MissingResponseBody)?;
    Ok(WebChannelImpl {
        _phantom: PhantomData,
        input: to_json_sink::<I>(),
        output: to_json_stream(body),
    })
}

pub trait WebChannel: Sized {
    type Input;
    type Output;

    fn split(
        self,
    ) -> (
        impl Sink<Self::Input, Error = std::io::Error>,
        impl Stream<Item = Result<Self::Output, JsonStreamError>>,
    );
}

struct WebChannelImpl<I, IS: Sink<I, Error = std::io::Error>, OS: Stream> {
    _phantom: PhantomData<I>,
    input: IS,
    output: OS,
}

impl<I, IS: Sink<I, Error = std::io::Error>, O, OS: Stream<Item = Result<O, JsonStreamError>>>
    WebChannel for WebChannelImpl<I, IS, OS>
{
    type Input = I;
    type Output = O;

    fn split(
        self,
    ) -> (
        impl Sink<Self::Input, Error = std::io::Error>,
        impl Stream<Item = Result<Self::Output, JsonStreamError>>,
    ) {
        (self.input, self.output)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WebChannelError {
    #[error("[{n}] Failed to open JSON stream: {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] There is no response body", n = self.name())]
    MissingResponseBody,
}
