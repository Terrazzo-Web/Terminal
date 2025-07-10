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
use crate::backend::protos::terrazzo::gateway::client::notify_response;
use crate::backend::protos::terrazzo::gateway::client::notify_response::FileEventKind as FileEventKindProto;
use crate::text_editor::notify::EventKind;
use crate::text_editor::notify::FileEventKind;
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
    Some(poll_next_remote_some(response?))
}

#[expect(clippy::result_large_err)]
fn poll_next_remote_some(
    response: Result<NotifyResponse, ServerFnError>,
) -> Result<NotifyResponseProto, Status> {
    let response = response.map_err(|error| Status::internal(format!("Remote error: {error}")))?;
    let event_kind = match response.kind {
        EventKind::File(kind) => notify_response::Kind::File(
            match kind {
                FileEventKind::Create => FileEventKindProto::Create,
                FileEventKind::Modify => FileEventKindProto::Modify,
                FileEventKind::Delete => FileEventKindProto::Delete,
                FileEventKind::Error => FileEventKindProto::Error,
            }
            .into(),
        ),
        EventKind::CargoCheck(diagnostics) => notify_response::Kind::CargoCheck(
            serde_json::to_string(&diagnostics)
                .map_err(|error| Status::internal(format!("JSON error: {error}")))?,
        ),
    };
    Ok(NotifyResponseProto {
        path: response.path,
        kind: event_kind.into(),
    })
}
