use std::os::fd::AsRawFd as _;
use std::os::fd::FromRawFd as _;
use std::os::unix::ffi::OsStrExt as _;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tracing::debug;

use super::raw_pts::Pts;
use super::size::Size;

#[derive(Debug)]
pub struct RawPty(std::os::fd::OwnedFd);

impl RawPty {
    pub fn open() -> Result<Self, OpenError> {
        let pt = rustix::pty::openpt(
            // Can't use CLOEXEC here because it's linux-specific
            rustix::pty::OpenptFlags::RDWR | rustix::pty::OpenptFlags::NOCTTY,
        )
        .map_err(OpenError::OpenPT)?;
        rustix::pty::grantpt(&pt).map_err(OpenError::GrantPT)?;
        rustix::pty::unlockpt(&pt).map_err(OpenError::UnlockPT)?;

        let mut flags = rustix::io::fcntl_getfd(&pt).map_err(OpenError::FcntlGetFD)?;
        flags |= rustix::io::FdFlags::CLOEXEC;
        rustix::io::fcntl_setfd(&pt, flags).map_err(OpenError::FcntlSetFD)?;
        Ok(Self(pt))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum OpenError {
    #[error("[{n}] {0}", n = self.name())]
    OpenPT(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    GrantPT(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    UnlockPT(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    FcntlGetFD(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    FcntlSetFD(rustix::io::Errno),
}

impl RawPty {
    pub fn set_term_size(&self, size: Size) -> Result<(), SetSizeError> {
        let size = libc::winsize::from(size);
        let fd = self.0.as_raw_fd();
        let ret = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, std::ptr::addr_of!(size)) };
        if ret == -1 {
            Err(rustix::io::Errno::from_raw_os_error(
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            )
            .into())
        } else {
            Ok(())
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetSizeError {
    #[error("[{n}] {0}", n = self.name())]
    IoCtlWinSize(#[from] rustix::io::Errno),
}

impl RawPty {
    pub fn pts(&self) -> Result<Pts, PtsError> {
        let ptsname = rustix::pty::ptsname(&self.0, vec![]).map_err(PtsError::PtsNameError)?;
        debug!(?ptsname, "pts");
        Ok(Pts(std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(std::ffi::OsStr::from_bytes(ptsname.as_bytes()))
            .map_err(PtsError::OpenError)?
            .into()))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PtsError {
    #[error("[{n}] {0}", n = self.name())]
    PtsNameError(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    OpenError(std::io::Error),
}

impl RawPty {
    pub fn set_nonblocking(&self) -> Result<(), SetNonBlockingError> {
        let mut opts = rustix::fs::fcntl_getfl(&self.0).map_err(SetNonBlockingError::FcntlGetFL)?;
        opts |= rustix::fs::OFlags::NONBLOCK;
        rustix::fs::fcntl_setfl(&self.0, opts).map_err(SetNonBlockingError::FcntlSetFL)?;
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetNonBlockingError {
    #[error("[{n}] {0}", n = self.name())]
    FcntlGetFL(rustix::io::Errno),

    #[error("[{n}] {0}", n = self.name())]
    FcntlSetFL(rustix::io::Errno),
}

impl From<RawPty> for std::os::fd::OwnedFd {
    fn from(pty: RawPty) -> Self {
        let RawPty(nix_ptymaster) = pty;
        let raw_fd = nix_ptymaster.as_raw_fd();
        std::mem::forget(nix_ptymaster);

        // Safety: nix::pty::PtyMaster is required to contain a valid file
        // descriptor, and we ensured that the file descriptor will remain
        // valid by skipping the drop implementation for nix::pty::PtyMaster
        unsafe { Self::from_raw_fd(raw_fd) }
    }
}

impl std::os::fd::AsFd for RawPty {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        let raw_fd = self.0.as_raw_fd();

        // Safety: nix::pty::PtyMaster is required to contain a valid file
        // descriptor, and it is owned by self
        unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) }
    }
}

impl std::os::fd::AsRawFd for RawPty {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }
}

impl std::io::Read for RawPty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        rustix::io::read(&self.0, buf).map_err(std::io::Error::from)
    }
}

impl std::io::Write for RawPty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        rustix::io::write(&self.0, buf).map_err(std::io::Error::from)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for &RawPty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        rustix::io::read(&self.0, buf).map_err(std::io::Error::from)
    }
}

impl std::io::Write for &RawPty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        rustix::io::write(&self.0, buf).map_err(std::io::Error::from)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
