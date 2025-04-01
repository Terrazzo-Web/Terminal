use std::marker::PhantomData;

use futures::Sink;
use futures::Stream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use serde::Serialize;
use web_sys::RequestInit;

use self::download::DownloadError;
use self::download::get_download_stream;
use self::upload::into_upload_stream;

pub mod download;
pub mod upload;

#[allow(unused)]
pub async fn open_channel<I, O, F, FF>(
    url: String,
    on_upload_request: impl FnOnce(&RequestInit),
    on_download_request: F,
) -> Result<impl WebChannel<Input = I, Output = O>, WebChannelError>
where
    I: Serialize,
    O: for<'t> Deserialize<'t>,
    F: Fn() -> FF,
    FF: FnOnce(&RequestInit),
{
    let (correlation_id, input) = into_upload_stream::<I>(&url, None.unwrap()).await;
    let output = get_download_stream(&url, correlation_id, on_download_request).await?;
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
        impl Stream<Item = Result<Self::Output, DownloadError>>,
    );
}

struct WebChannelImpl<I, IS: Sink<I, Error = std::io::Error>, OS: Stream> {
    _phantom: PhantomData<I>,
    input: IS,
    output: OS,
}

impl<I, IS: Sink<I, Error = std::io::Error>, O, OS: Stream<Item = Result<O, DownloadError>>>
    WebChannel for WebChannelImpl<I, IS, OS>
{
    type Input = I;
    type Output = O;

    fn split(
        self,
    ) -> (
        impl Sink<Self::Input, Error = std::io::Error>,
        impl Stream<Item = Result<Self::Output, DownloadError>>,
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
