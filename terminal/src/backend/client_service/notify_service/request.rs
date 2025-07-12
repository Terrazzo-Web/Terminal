use pin_project::pin_project;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Streaming;

use crate::backend::protos::terrazzo::notify::NotifyRequest as NotifyRequestProto;
use crate::text_editor::notify::NotifyRequest;

pub mod local;
pub mod remote;

#[pin_project(project = HybridRequestStreamProj)]
pub enum HybridRequestStream {
    Local(BoxedStream<NotifyRequest, ServerFnError>),
    Remote(#[pin] Box<Streaming<NotifyRequestProto>>),
}

impl From<HybridRequestStream> for BoxedStream<NotifyRequest, ServerFnError> {
    fn from(request_stream: HybridRequestStream) -> Self {
        match request_stream {
            HybridRequestStream::Local(local_stream) => local_stream,
            request_stream => self::local::LocalRequestStream(request_stream).into(),
        }
    }
}

impl From<BoxedStream<NotifyRequest, ServerFnError>> for HybridRequestStream {
    fn from(request_stream: BoxedStream<NotifyRequest, ServerFnError>) -> Self {
        Self::Local(request_stream)
    }
}

impl From<Streaming<NotifyRequestProto>> for HybridRequestStream {
    fn from(request_stream: Streaming<NotifyRequestProto>) -> Self {
        Self::Remote(Box::new(request_stream))
    }
}
