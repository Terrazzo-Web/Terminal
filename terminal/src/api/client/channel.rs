use std::marker::PhantomData;

use futures::Sink;
use futures::Stream;
use json_stream::DownloadError;
use json_stream::JsonStreamError;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use web_sys::RequestInit;

use self::json_sink::to_json_sink;
use self::json_stream::to_json_stream;

pub mod json_sink;
pub mod json_stream;

#[allow(unused)]
pub async fn open_channel<I, O, F, FF>(
    url: String,
    on_request: F,
) -> Result<impl WebChannel<Input = I, Output = O>, WebChannelError>
where
    O: for<'t> Deserialize<'t>,
    F: Fn() -> FF,
    FF: FnOnce(&RequestInit),
{
    let (correlation_id, input) = to_json_sink::<I>();
    let output = to_json_stream(&url, correlation_id, on_request).await?;
    Ok(WebChannelImpl {
        _phantom: PhantomData,
        input,
        output,
    })
}

#[allow(unused)]
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
    #[error("[{n}] {0}", n = self.name())]
    Download(#[from] DownloadError),
}
