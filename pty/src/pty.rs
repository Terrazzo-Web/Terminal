use std::io::Read as _;
use std::io::Write as _;
use std::sync::Arc;

use super::raw_pts::Pts;
use super::raw_pty;
use super::raw_pty::PtsError;
use super::size::Size;

type AsyncPty = tokio::io::unix::AsyncFd<raw_pty::RawPty>;

/// An allocated pty
pub struct Pty(AsyncPty);

#[derive(thiserror::Error, Debug)]
pub enum PtyError {
    #[error("OpenError: {0}")]
    OpenError(#[from] raw_pty::OpenError),

    #[error("SetSizeError: {0}")]
    SetSizeError(#[from] raw_pty::SetSizeError),

    #[error("SetNonBlockingError: {0}")]
    SetNonBlockingError(#[from] raw_pty::SetNonBlockingError),

    #[error("AsyncFdError: {0}")]
    AsyncFdError(std::io::Error),

    #[error("PtsError: {0}")]
    PtsError(#[from] PtsError),
}

impl Pty {
    pub fn new() -> Result<Self, PtyError> {
        let pty = raw_pty::RawPty::open()?;
        let async_fd = tokio::io::unix::AsyncFd::new(pty);
        Ok(Self(async_fd.map_err(PtyError::AsyncFdError)?))
    }

    pub fn resize(&self, size: Size) -> Result<(), PtyError> {
        Ok(self.0.get_ref().set_term_size(size)?)
    }

    pub fn pts(&self) -> Result<Pts, PtyError> {
        Ok(self.0.get_ref().pts()?)
    }

    pub fn set_nonblocking(&self) -> Result<(), PtyError> {
        Ok(self.0.get_ref().set_nonblocking()?)
    }

    #[must_use]
    pub fn into_split(self) -> (OwnedReadPty, OwnedWritePty) {
        let pt = Arc::new(self.0);
        (OwnedReadPty(pt.clone()), OwnedWritePty(pt.clone()))
    }
}

impl From<Pty> for std::os::fd::OwnedFd {
    fn from(pty: Pty) -> Self {
        pty.0.into_inner().into()
    }
}

impl std::os::fd::AsFd for Pty {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.0.get_ref().as_fd()
    }
}

impl std::os::fd::AsRawFd for Pty {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.get_ref().as_raw_fd()
    }
}

/// Owned read half of a [`Pty`]
#[derive(Debug)]
pub struct OwnedReadPty(std::sync::Arc<AsyncPty>);

impl tokio::io::AsyncRead for OwnedReadPty {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            let mut guard = match self.0.poll_read_ready(cx) {
                std::task::Poll::Ready(guard) => guard,
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }?;
            let b = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_ref().read(b)) {
                Ok(Ok(bytes)) => {
                    buf.advance(bytes);
                    return std::task::Poll::Ready(Ok(()));
                }
                Ok(Err(e)) => return std::task::Poll::Ready(Err(e)),
                Err(_would_block) => continue,
            }
        }
    }
}

/// Owned write half of a [`Pty`]
#[derive(Debug)]
pub struct OwnedWritePty(std::sync::Arc<AsyncPty>);

impl OwnedWritePty {
    /// Change the terminal size associated with the pty.
    ///
    /// # Errors
    /// Returns an error if we were unable to set the terminal size.
    pub fn resize(&self, size: super::size::Size) -> Result<(), PtyError> {
        Ok(self.0.get_ref().set_term_size(size)?)
    }
}

impl tokio::io::AsyncWrite for OwnedWritePty {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        loop {
            let mut guard = match self.0.poll_write_ready(cx) {
                std::task::Poll::Ready(guard) => guard,
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }?;
            match guard.try_io(|inner| inner.get_ref().write(buf)) {
                Ok(result) => return std::task::Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            let mut guard = match self.0.poll_write_ready(cx) {
                std::task::Poll::Ready(guard) => guard,
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }?;
            match guard.try_io(|inner| inner.get_ref().flush()) {
                Ok(_) => return std::task::Poll::Ready(Ok(())),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
