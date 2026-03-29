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

#[pin_project(project = RemoteResponseStreamProj)]
pub struct RemoteResponseStream(#[pin] pub HybridResponseStream);

impl futures::Stream for RemoteResponseStream {
    type Item = Result<LogsResponseProto, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().0.project() {
            HybridResponseStreamProj::Local(this) => {
                poll_next_remote(ready!(this.as_mut().poll_next(cx))).into()
            }
            HybridResponseStreamProj::Remote(this) => this.poll_next(cx),
        }
    }
}

fn poll_next_remote(
    response: Option<Result<LogEvent, ServerFnError>>,
) -> Option<Result<LogsResponseProto, Status>> {
    Some(poll_next_remote_some(response?))
}

fn poll_next_remote_some(
    response: Result<LogEvent, ServerFnError>,
) -> Result<LogsResponseProto, Status> {
    let response = response.map_err(|error| Status::internal(format!("Remote error: {error}")))?;
    Ok(LogsResponseProto {
        id: response.id,
        level: match response.level {
            LogLevel::Info => LogLevelProto::Info,
            LogLevel::Warn => LogLevelProto::Warn,
            LogLevel::Error => LogLevelProto::Error,
            LogLevel::Debug => LogLevelProto::Debug,
        }
        .into(),
        message: response.message,
        timestamp_ms: response.timestamp_ms,
        file: response.file,
    })
}
