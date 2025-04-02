use download::DownloadError;
use futures::Stream;
use futures::channel::oneshot;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use serde::Deserialize;
use serde::Serialize;
use web_sys::RequestInit;

use self::download::DownloadItemError;
use self::download::get_download_stream;
use self::upload::into_upload_stream;

pub mod download;
pub mod upload;

#[allow(unused)]
pub async fn open_channel<I, O, F, FF>(
    url: &str,
    on_upload_request: impl FnOnce(&RequestInit) + 'static,
    on_download_request: F,
    upload: impl Stream<Item = O> + 'static,
) -> Result<impl Stream<Item = Result<I, DownloadItemError>>, WebChannelError>
where
    I: for<'t> Deserialize<'t>,
    O: Serialize,
    F: Fn() -> FF,
    FF: FnOnce(&RequestInit),
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
}
