use named::named;
use named::NamedEnumValues as _;
use web_sys::Response;

use super::pipe::PipeError;
use crate::api::client::send_request;
use crate::api::client::Method;
use crate::api::client::SendRequestError;
use crate::api::client::BASE_URL;
use crate::api::RegisterTerminalQuery;
use crate::terminal_id::TerminalId;

/// Instructs the server to include `terminal_id`'s data in the pipe.
#[named]
pub async fn register(
    terminal_id: &TerminalId,
    query: RegisterTerminalQuery,
) -> Result<(), RegisterError> {
    let _: Response = send_request(
        Method::POST,
        format!(
            "{BASE_URL}/stream/{REGISTER}/{terminal_id}?{query}",
            query = serde_urlencoded::to_string(query).unwrap()
        ),
        move |_| {},
    )
    .await?;
    return Ok(());
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum RegisterError {
    #[error("[{n}] {0}", n = self.name())]
    SendRequestError(#[from] SendRequestError),

    #[error("[{n}] {0}", n = self.name())]
    PipeError(#[from] PipeError),
}

#[cfg(test)]
mod tests {
    use crate::api::RegisterTerminalMode;
    use crate::api::RegisterTerminalQuery;

    #[test]
    fn serialize_register_terminal_mode() {
        let query = RegisterTerminalQuery {
            mode: RegisterTerminalMode::Create,
        };
        assert_eq!("mode=Create", serde_urlencoded::to_string(&query).unwrap());
    }
}
