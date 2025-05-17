use nameth::NamedEnumValues as _;
use nameth::nameth;
use web_sys::Response;

use super::request::Method;
use super::request::SendRequestError;
use super::request::send_request;
use super::request::set_json_body;

#[nameth]
pub async fn login(password: Option<&str>) -> Result<(), LoginError> {
    let _: Response = send_request(
        Method::POST,
        format!("/api/{LOGIN} "),
        set_json_body(&password)?,
    )
    .await?;
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    InvalidJson(#[from] serde_json::Error),
}
