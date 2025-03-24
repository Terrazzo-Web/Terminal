#![doc = include_str!("../README.md")]

use std::task::Poll;
use std::task::ready;

use futures::Stream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use pin_project::pin_project;
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;

use self::command::Command;
use self::command::SpawnError;
use self::pty::OwnedReadPty;
use self::pty::OwnedWritePty;
use self::pty::Pty;
use self::pty::PtyError;
use self::size::Size;

mod command;
pub mod lease;
mod pty;
mod raw_pts;
mod raw_pty;
mod release_on_drop;
mod size;

const BUFFER_SIZE: usize = 1024;

pub struct ProcessIO<W = OwnedWritePty, R = ReaderStream<OwnedReadPty>> {
    input: W,
    output: R,
    #[expect(unused)]
    child_process: tokio::process::Child,
}

#[pin_project]
pub struct ProcessInput<W = OwnedWritePty>(#[pin] W);

#[pin_project]
pub struct ProcessOutput<R = ReaderStream<OwnedReadPty>>(#[pin] R);

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum OpenProcessError {
    #[error("[{n}] {0}", n = self.name())]
    PtyProcessError(#[from] PtyError),

    #[error("[{n}] {0}", n = self.name())]
    SpawnError(#[from] SpawnError),

    #[error("[{n}] Not found", n = self.name())]
    NotFound,
}

impl ProcessIO<OwnedWritePty, ReaderStream<OwnedReadPty>> {
    pub async fn open() -> Result<Self, OpenProcessError> {
        let pty = Pty::new()?;
        let mut command =
            std::env::var("SHELL").map_or_else(|_| Command::new("/bin/bash"), Command::new);
        command.arg("-i");
        let child = command.spawn(&pty.pts()?)?;

        // https://forums.developer.apple.com/forums/thread/734230
        pty.set_nonblocking()?;

        return Ok(Self::new(pty, child));
    }

    fn new(pty: Pty, child_process: tokio::process::Child) -> Self {
        let (output, input) = pty.into_split();
        let output = ReaderStream::with_capacity(output, BUFFER_SIZE);
        Self {
            input,
            output,
            child_process,
        }
    }
}

impl<W, R> ProcessIO<W, R> {
    pub fn split(self) -> (ProcessInput<W>, ProcessOutput<R>) {
        (ProcessInput(self.input), ProcessOutput(self.output))
    }
}

impl ProcessInput<OwnedWritePty> {
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<(), ResizeTerminalError> {
        self.0.resize(Size::new(rows, cols))?;
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ResizeTerminalError {
    #[error("[{n}] {0}", n = self.name())]
    PtyError(#[from] PtyError),
}

impl<W: tokio::io::AsyncWrite> tokio::io::AsyncWrite for ProcessInput<W> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().0.poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.0.is_write_vectored()
    }
}

impl<R: Stream<Item = std::io::Result<D>>, D: IsData> Stream for ProcessOutput<R> {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().0.poll_next(cx)) {
            Some(Ok(bytes)) if bytes.has_data() => Some(Ok(bytes.into_vec())),
            Some(Err(error)) => Some(Err(error)),
            _ => None,
        }
        .into()
    }
}

pub trait IsData {
    fn has_data(&self) -> bool;
    fn into_vec(self) -> Vec<u8>;
}

pub trait IsDataStream: Stream<Item = std::io::Result<Self::Data>> {
    type Data: IsData;
}

impl<S: Stream<Item = std::io::Result<D>>, D: IsData> IsDataStream for S {
    type Data = D;
}

impl IsData for Bytes {
    fn has_data(&self) -> bool {
        !Bytes::is_empty(self)
    }

    fn into_vec(self) -> Vec<u8> {
        self.into()
    }
}

impl IsData for Vec<u8> {
    fn has_data(&self) -> bool {
        !self.is_empty()
    }

    fn into_vec(self) -> Vec<u8> {
        self
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn open() {
        super::ProcessIO::open().await.unwrap();
    }
}
