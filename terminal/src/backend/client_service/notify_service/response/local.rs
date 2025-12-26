use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use server_fn::ServerFnError;
use tonic::Status;

use super::HybridResponseStream;
use super::HybridResponseStreamProj;
use crate::backend::protos::terrazzo::notify::NotifyResponse as NotifyResponseProto;
use crate::backend::protos::terrazzo::notify::notify_response;
use crate::backend::protos::terrazzo::notify::notify_response::FileEventKind as FileEventKindProto;
use crate::text_editor::notify::server_fn::EventKind;
use crate::text_editor::notify::server_fn::FileEventKind;
use crate::text_editor::notify::server_fn::NotifyResponse;

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
    Some(poll_next_local_some(response?))
}

fn poll_next_local_some(
    response: Result<NotifyResponseProto, Status>,
) -> Result<NotifyResponse, ServerFnError> {
    let response = response?;
    let kind = match response.kind {
        Some(kind) => match kind {
            notify_response::Kind::File(kind) => EventKind::File({
                match FileEventKindProto::try_from(kind).unwrap_or_default() {
                    FileEventKindProto::Error => FileEventKind::Error,
                    FileEventKindProto::Create => FileEventKind::Create,
                    FileEventKindProto::Modify => FileEventKind::Modify,
                    FileEventKindProto::Delete => FileEventKind::Delete,
                }
            }),
            notify_response::Kind::CargoCheck(diagnostics) => {
                EventKind::CargoCheck(serde_json::from_str(&diagnostics)?)
            }
        },
        None => EventKind::File(FileEventKind::Error),
    };
    Ok(NotifyResponse {
        path: response.path,
        kind,
    })
}
