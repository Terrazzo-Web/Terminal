use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use futures::Stream;
use pin_project::pin_project;
use terrazzo_pty::lease::LeaseItem;
use terrazzo_pty::lease::ProcessOutputLease;
use tonic::Status;
use tonic::Streaming;

use crate::backend::protos::terrazzo::gateway::client::LeaseItem as LeaseItemProto;
use crate::backend::protos::terrazzo::gateway::client::lease_item;

#[pin_project(project = HybridReaderProj)]
pub enum HybridReader {
    Local(#[pin] ProcessOutputLease),
    Remote(#[pin] Streaming<LeaseItemProto>),
}

#[pin_project(project = LocalReaderProj)]
pub struct LocalReader(#[pin] pub HybridReader);

#[pin_project(project = RemoteReaderProj)]
pub struct RemoteReader(#[pin] pub HybridReader);

impl Stream for LocalReader {
    type Item = <ProcessOutputLease as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let LocalReaderProj(reader) = self.project();
        match reader.project() {
            HybridReaderProj::Local(reader) => reader.poll_next(cx),
            HybridReaderProj::Remote(reader) => Poll::Ready(match ready!(reader.poll_next(cx)) {
                Some(Ok(LeaseItemProto {
                    kind: Some(lease_item::Kind::Data(data)),
                })) => Some(LeaseItem::Data(data)),

                Some(Ok(LeaseItemProto { kind: None })) => Some(LeaseItem::EOS),
                Some(Ok(LeaseItemProto {
                    kind: Some(lease_item::Kind::Eos { .. }),
                })) => Some(LeaseItem::EOS),

                Some(Err(error)) => Some(LeaseItem::Error(std::io::Error::new(
                    ErrorKind::ConnectionAborted,
                    error,
                ))),

                None => None,
            }),
        }
    }
}

impl Stream for RemoteReader {
    type Item = <Streaming<LeaseItemProto> as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let RemoteReaderProj(reader) = self.project();
        match reader.project() {
            HybridReaderProj::Local(reader) => Poll::Ready(match ready!(reader.poll_next(cx)) {
                Some(LeaseItem::EOS) => Some(Ok(LeaseItemProto {
                    kind: Some(lease_item::Kind::Eos(true)),
                })),

                Some(LeaseItem::Data(data)) => Some(Ok(LeaseItemProto {
                    kind: Some(lease_item::Kind::Data(data)),
                })),

                Some(LeaseItem::Error(data)) => Some(Err(Status::aborted(data.to_string()))),

                None => None,
            }),
            HybridReaderProj::Remote(reader) => reader.poll_next(cx),
        }
    }
}
