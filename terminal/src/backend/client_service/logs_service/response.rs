use pin_project::pin_project;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Streaming;

use crate::backend::protos::terrazzo::logs::LogsResponse as LogsResponseProto;
use crate::logs::event::LogEvent;

pub mod local;
pub mod remote;

#[pin_project(project = HybridResponseStreamProj)]
pub enum HybridResponseStream {
    Local(BoxedStream<LogEvent, ServerFnError>),
    Remote(#[pin] Box<Streaming<LogsResponseProto>>),
}

impl From<HybridResponseStream> for BoxedStream<LogEvent, ServerFnError> {
    fn from(response_stream: HybridResponseStream) -> Self {
        match response_stream {
            HybridResponseStream::Local(local_stream) => local_stream,
            response_stream => self::local::LocalResponseStream(response_stream).into(),
        }
    }
}
