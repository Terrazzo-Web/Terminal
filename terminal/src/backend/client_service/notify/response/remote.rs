use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use server_fn::ServerFnError;
use tonic::Status;

use super::HybridResponseStream;
use super::HybridResponseStreamProj;
use crate::backend::protos::terrazzo::gateway::client::NotifyResponse as NotifyResponseProto;
use crate::backend::protos::terrazzo::gateway::client::notify_response::EventKind as EventKindProto;
use crate::text_editor::notify::EventKind;
use crate::text_editor::notify::NotifyResponse;

#[pin_project(project = RemoteReaderProj)]
pub struct RemoteResponseStream(#[pin] pub HybridResponseStream);

impl futures::Stream for RemoteResponseStream {
    type Item = Result<NotifyResponseProto, Status>;

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
    response: Option<Result<NotifyResponse, ServerFnError>>,
) -> Option<Result<NotifyResponseProto, Status>> {
    Some(
        response?
            .map(|response| {
                let event_kind = match response.kind {
                    EventKind::Create => EventKindProto::Create,
                    EventKind::Modify => EventKindProto::Modify,
                    EventKind::Delete => EventKindProto::Delete,
                    EventKind::Error => EventKindProto::Error,
                    EventKind::Lang => EventKindProto::Lang,
                };
                NotifyResponseProto {
                    path: response.path,
                    kind: event_kind.into(),
                }
            })
            .map_err(|error| Status::internal(format!("Remote error: {error}"))),
    )
}
