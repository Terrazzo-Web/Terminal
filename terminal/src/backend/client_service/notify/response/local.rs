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

#[pin_project(project = LocalResponseStreamProj)]
pub struct LocalResponseStream(#[pin] pub HybridResponseStream);

impl futures::Stream for LocalResponseStream {
    type Item = Result<NotifyResponse, ServerFnError>;

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
    response: Option<Result<NotifyResponseProto, Status>>,
) -> Option<Result<NotifyResponse, ServerFnError>> {
    Some(
        response?
            .map(|response| {
                let event_kind = match response.kind() {
                    EventKindProto::Create => EventKind::Create,
                    EventKindProto::Modify => EventKind::Modify,
                    EventKindProto::Delete => EventKind::Delete,
                    EventKindProto::Error => EventKind::Error,
                };
                NotifyResponse {
                    path: response.path,
                    kind: event_kind,
                }
            })
            .map_err(Status::into),
    )
}
