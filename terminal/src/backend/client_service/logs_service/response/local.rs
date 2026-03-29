use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use server_fn::ServerFnError;
use tonic::Status;

use super::HybridResponseStream;
use super::HybridResponseStreamProj;
use crate::backend::protos::terrazzo::logs::LogLevel as LogLevelProto;
use crate::backend::protos::terrazzo::logs::LogsResponse as LogsResponseProto;
use crate::logs::event::LogEvent;
use crate::logs::event::LogLevel;

#[pin_project(project = LocalResponseStreamProj)]
pub struct LocalResponseStream(#[pin] pub HybridResponseStream);

impl futures::Stream for LocalResponseStream {
    type Item = Result<LogEvent, ServerFnError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().0.project() {
            HybridResponseStreamProj::Local(this) => this.as_mut().poll_next(cx),
            HybridResponseStreamProj::Remote(this) => {
                poll_next_local(ready!(this.poll_next(cx))).into()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

fn poll_next_local(
    response: Option<Result<LogsResponseProto, Status>>,
) -> Option<Result<LogEvent, ServerFnError>> {
    Some(poll_next_local_some(response?))
}

fn poll_next_local_some(
    response: Result<LogsResponseProto, Status>,
) -> Result<LogEvent, ServerFnError> {
    let response = response?;
    Ok(LogEvent {
        id: response.id,
        level: match LogLevelProto::try_from(response.level).unwrap_or(LogLevelProto::Info) {
            LogLevelProto::Info => LogLevel::Info,
            LogLevelProto::Warn => LogLevel::Warn,
            LogLevelProto::Error => LogLevel::Error,
            LogLevelProto::Debug => LogLevel::Debug,
        },
        message: response.message,
        timestamp_ms: response.timestamp_ms,
        file: response.file,
    })
}
