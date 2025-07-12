use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use server_fn::ServerFnError;
use tonic::Status;

use super::HybridRequestStream;
use super::HybridRequestStreamProj;
use crate::backend::protos::terrazzo::notify::NotifyRequest as NotifyRequestProto;
use crate::backend::protos::terrazzo::notify::notify_request::RequestType as RequestTypeProto;
use crate::text_editor::notify::NotifyRequest;

#[pin_project(project = LocalRequestStreamProj)]
pub struct LocalRequestStream(#[pin] pub HybridRequestStream);

impl futures::Stream for LocalRequestStream {
    type Item = Result<NotifyRequest, ServerFnError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().0.project() {
            HybridRequestStreamProj::Local(this) => this.as_mut().poll_next(cx),
            HybridRequestStreamProj::Remote(this) => {
                poll_next_local(ready!(this.poll_next(cx))).into()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

fn poll_next_local(
    request: Option<Result<NotifyRequestProto, Status>>,
) -> Option<Result<NotifyRequest, ServerFnError>> {
    Some(
        request?
            .map(|request| match request.request_type.unwrap() {
                RequestTypeProto::Address(remote) => NotifyRequest::Start {
                    remote: remote.into(),
                },
                RequestTypeProto::Watch(full_path) => NotifyRequest::Watch {
                    full_path: full_path.into(),
                },
                RequestTypeProto::Unwatch(full_path) => NotifyRequest::UnWatch {
                    full_path: full_path.into(),
                },
            })
            .map_err(Status::into),
    )
}
