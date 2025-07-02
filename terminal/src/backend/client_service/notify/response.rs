use pin_project::pin_project;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Streaming;

use crate::backend::protos::terrazzo::gateway::client::NotifyResponse as NotifyResponseProto;
use crate::text_editor::notify::NotifyResponse;

pub mod local;
pub mod remote;

#[pin_project(project = HybridResponseStreamProj)]
pub enum HybridResponseStream {
    Local(BoxedStream<NotifyResponse, ServerFnError>),
    Remote(#[pin] Streaming<NotifyResponseProto>),
}
