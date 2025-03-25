use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::Stream;
use pin_project::pin_project;
use terrazzo_pty::pty::OwnedReadPty;
use terrazzo_pty::pty::OwnedWritePty;
use tokio_util::io::ReaderStream;

#[pin_project(project = PtyWriterProj)]
pub enum PtyWriter {
    Local(#[pin] OwnedWritePty),
    #[expect(unused)]
    Remote(#[pin] OwnedWritePty),
}

impl tokio::io::AsyncWrite for PtyWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.project() {
            PtyWriterProj::Local(writer) => writer.poll_write(cx, buf),
            PtyWriterProj::Remote(writer) => writer.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            PtyWriterProj::Local(writer) => writer.poll_flush(cx),
            PtyWriterProj::Remote(writer) => writer.poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            PtyWriterProj::Local(writer) => writer.poll_shutdown(cx),
            PtyWriterProj::Remote(writer) => writer.poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.project() {
            PtyWriterProj::Local(writer) => writer.poll_write_vectored(cx, bufs),
            PtyWriterProj::Remote(writer) => writer.poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Local(writer) => writer.is_write_vectored(),
            Self::Remote(writer) => writer.is_write_vectored(),
        }
    }
}

#[pin_project(project = PtyReaderProj)]
pub enum PtyReader {
    Local(#[pin] ReaderStream<OwnedReadPty>),
    #[expect(unused)]
    Remote(#[pin] ReaderStream<OwnedReadPty>),
}

impl Stream for PtyReader {
    type Item = <ReaderStream<OwnedReadPty> as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            PtyReaderProj::Local(reader) => reader.poll_next(cx),
            PtyReaderProj::Remote(reader) => reader.poll_next(cx),
        }
    }
}
