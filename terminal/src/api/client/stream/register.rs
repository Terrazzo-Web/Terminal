use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use crate::api::RegisterTerminalRequest;
use crate::api::TerminalDef;
use crate::api::client::request::BASE_URL;
use crate::api::client::request::Method;
use crate::api::client::request::SendRequestError;
use crate::api::client::request::send_request;
use crate::api::client::request::set_json_body;

/// Instructs the server to include `terminal_id`'s data in the pipe.
#[nameth]
pub async fn register(request: RegisterTerminalRequest<&TerminalDef>) -> Result<(), RegisterError> {
    let _: Response = send_request(
        Method::POST,
        &format!("{BASE_URL}/stream/{REGISTER}"),
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
    InvalidJson(#[from] serde_json::Error),
}
