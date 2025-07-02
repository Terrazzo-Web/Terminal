use pin_project::pin_project;
use server_fn::BoxedStream;
use server_fn::ServerFnError;
use tonic::Streaming;

use crate::backend::protos::terrazzo::gateway::client::NotifyRequest as NotifyRequestProto;
use crate::text_editor::notify::NotifyRequest;

pub mod local;
pub mod remote;

#[pin_project(project = HybridRequestStreamProj)]
pub enum HybridRequestStream {
    Local(BoxedStream<NotifyRequest, ServerFnError>),
    Remote(#[pin] Streaming<NotifyRequestProto>),
}
