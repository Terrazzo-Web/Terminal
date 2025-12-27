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
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;
use crate::text_editor::notify::server_fn::NotifyRequest;

#[pin_project(project = RemoteReaderProj)]
pub struct RemoteRequestStream(#[pin] pub HybridRequestStream);

impl futures::Stream for RemoteRequestStream {
    type Item = Result<NotifyRequestProto, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().0.project() {
            HybridRequestStreamProj::Local(this) => {
                poll_next_remote(ready!(this.as_mut().poll_next(cx))).into()
            }
            HybridRequestStreamProj::Remote(this) => this.poll_next(cx),
        }
    }
}

fn poll_next_remote(
    request: Option<Result<NotifyRequest, ServerFnError>>,
) -> Option<Result<NotifyRequestProto, Status>> {
    Some(
        request?
            .map(|request| NotifyRequestProto {
                request_type: Some(match request {
                    NotifyRequest::Start { remote } => {
                        RequestTypeProto::Address(ClientAddressProto::of(&remote))
                    }
                    NotifyRequest::Watch { full_path } => RequestTypeProto::Watch(full_path.into()),
                    NotifyRequest::UnWatch { full_path } => {
                        RequestTypeProto::Unwatch(full_path.into())
                    }
                }),
            })
            .map_err(|error| Status::internal(format!("Remote error: {error}"))),
    )
}
