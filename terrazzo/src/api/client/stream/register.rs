use named::named;
use named::NamedEnumValues as _;
use web_sys::Response;

use super::pipe::PipeError;
use crate::api::client::send_request;
use crate::api::client::Method;
use crate::api::client::SendRequestError;
use crate::api::client::BASE_URL;
use crate::terminal_id::TerminalId;

/// Instructs the server to include `terminal_id`'s data in the pipe.
#[named]
pub async fn register(terminal_id: &TerminalId) -> Result<(), RegisterError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_URL}/stream/{REGISTER}/{terminal_id}"),
        move |_| {},
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum RegisterError {
    #[error("[{}] {0}", self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{}] {0}", self.name())]
    PipeError(#[from] PipeError),
}
