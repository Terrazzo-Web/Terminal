use futures::Stream;
use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use serde::Serialize;
use web_sys::RequestInit;

use self::download::DownloadError;
use self::download::DownloadItemError;
use self::download::get_download_stream;
use self::upload::into_upload_stream;

pub mod download;
pub mod upload;

#[allow(unused)]
pub async fn open_channel<I, O, FI, FO, FFO, SO>(
    url: &str,
    on_upload_request: FI,
    on_download_request: FO,
    upload: SO,
) -> Result<
    impl Stream<Item = Result<I, DownloadItemError>> + use<I, O, FI, FO, FFO, SO>,
    WebChannelError,
>
where
    I: for<'t> Deserialize<'t>,
    O: Serialize,
    FI: FnOnce(&RequestInit) + 'static,
    FO: Fn() -> FFO,
    FFO: FnOnce(&RequestInit),
    SO: Stream<Item = O> + 'static,
{
    let (end_of_upload_tx, end_of_upload_rx) = oneshot::channel();
    let (end_of_download_tx, end_of_download_rx) = oneshot::channel();
    let correlation_id = into_upload_stream::<O>(
        url,
        on_upload_request,
        upload,
        end_of_upload_tx,
        end_of_download_rx,
    );
    Ok(get_download_stream(
        url,
        correlation_id,
        on_download_request,
        end_of_upload_rx,
        end_of_download_tx,
    )
    .await?)
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum WebChannelError {
    #[error("[{n}] {0}", n = self.name())]
    Download(#[from] DownloadError),

    #[error("[{n}] The channel was opened twice", n = self.name())]
    Race,
}
