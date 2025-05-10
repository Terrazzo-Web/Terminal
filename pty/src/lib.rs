#![doc = include_str!("../README.md")]

use std::task::Poll;
use std::task::ready;

use bytes::Bytes;
use futures::Stream;
use nameth::NamedEnumValues as _;
use nameth::nameth;
use pin_project::pin_project;
use tokio_util::io::ReaderStream;

use self::command::Command;
use self::command::SpawnError;
use self::pty::OwnedWritePty;
use self::pty::Pty;
use self::pty::PtyError;
use self::tail::TailStream;

mod command;
pub mod lease;
pub mod pty;
mod raw_pts;
mod raw_pty;
mod release_on_drop;
pub mod size;
mod tail;

const BUFFER_SIZE: usize = 1024;
const SCROLLBACK: usize = 1000;

pub struct ProcessIO {
    input: OwnedWritePty,
    output: TailStream,
    #[expect(unused)]
    child_process: tokio::process::Child,
}

#[pin_project]
pub struct ProcessInput(#[pin] pub OwnedWritePty);

#[pin_project]
pub struct ProcessOutput(#[pin] pub TailStream);

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

impl ProcessIO {
    pub async fn open(client_name: Option<impl AsRef<str>>) -> Result<Self, OpenProcessError> {
        let pty = Pty::new()?;
        let mut command =
            std::env::var("SHELL").map_or_else(|_| Command::new("/bin/bash"), Command::new);
        command.arg("-i");
        if let Some(client_name) = client_name {
            command.env("TERRAZZO_CLIENT_NAME", client_name.as_ref());
        }
        let child = command.spawn(&pty.pts()?)?;

        // https://forums.developer.apple.com/forums/thread/734230
        pty.set_nonblocking()?;

        return Ok(Self::new(pty, child));
    }

    fn new(pty: Pty, child_process: tokio::process::Child) -> Self {
        let (output, input) = pty.into_split();
        let output = ReaderStream::with_capacity(output, BUFFER_SIZE);
        let output = TailStream::new(output, SCROLLBACK);
        Self {
            input,
            output,
            child_process,
        }
    }

    pub fn split(self) -> (ProcessInput, ProcessOutput) {
        (ProcessInput(self.input), ProcessOutput(self.output))
    }
}

impl tokio::io::AsyncWrite for ProcessInput {
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

impl Stream for ProcessOutput {
    type Item = std::io::Result<Bytes>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().0.poll_next(cx)) {
            Some(Ok(bytes)) if !bytes.is_empty() => Some(Ok(bytes)),
            Some(Err(error)) => Some(Err(error)),
            _ => None,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn open() {
        super::ProcessIO::open(Option::<String>::None)
            .await
            .unwrap();
    }
}
