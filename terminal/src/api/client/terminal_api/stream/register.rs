use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::pipe::PipeError;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;
use crate::api::client::terminal_api::BASE_TERMINAL_URL;
use crate::api::shared::terminal_schema::RegisterTerminalRequest;

/// Instructs the server to include `terminal_id`'s data in the pipe.
#[nameth]
pub async fn register(request: RegisterTerminalRequest) -> Result<(), RegisterError> {
    let _: Response = send_request(
        Method::POST,
        format!("{BASE_TERMINAL_URL}/stream/{REGISTER}"),
        set_json_body(&request)?,
    )
    .await?;
    return Ok(());
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum RegisterError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    PipeError(#[from] PipeError),

    #[error("[{n}] {0}", n = self.name())]
    InvalidJson(#[from] serde_json::Error),
}
