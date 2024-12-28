use std::task::ready;
use std::task::Poll;

use pin_project::pin_project;
use tokio::io::AsyncWrite;
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

pub struct ProcessIO {
    input: OwnedWritePty,
    output: ReaderStream<OwnedReadPty>,
    #[expect(unused)]
    child_process: tokio::process::Child,
}

#[pin_project]
pub struct ProcessInput(#[pin] OwnedWritePty);

#[pin_project]
pub struct ProcessOutput(#[pin] ReaderStream<OwnedReadPty>);

#[derive(thiserror::Error, Debug)]
pub enum OpenProcessError {
    #[error("PtyProcessError: {0}")]
    PtyProcessError(#[from] PtyError),

    #[error("SpawnError: {0}")]
    SpawnError(#[from] SpawnError),
}

impl ProcessIO {
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

    pub fn split(self) -> (ProcessInput, ProcessOutput) {
        (ProcessInput(self.input), ProcessOutput(self.output))
    }
}

impl ProcessInput {
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<(), ResizeTerminalError> {
        self.0.resize(Size::new(rows, cols))?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ResizeTerminalError {
    #[error("PtyError: {0}")]
    PtyError(#[from] PtyError),
}

impl AsyncWrite for ProcessInput {
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

impl futures::Stream for ProcessOutput {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().0.poll_next(cx)) {
            Some(Ok(bytes)) => Some(Ok(bytes.to_vec())),
            Some(Err(error)) => Some(Err(error)),
            None => None,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn open() {
        super::ProcessIO::open().await.unwrap();
    }
}
