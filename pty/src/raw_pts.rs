use std::os::fd::AsRawFd as _;

pub struct Pts(pub(super) std::os::fd::OwnedFd);

impl Pts {
    pub fn setup_subprocess(
        &self,
    ) -> std::io::Result<(
        std::process::Stdio,
        std::process::Stdio,
        std::process::Stdio,
    )> {
        Ok((
            self.0.try_clone()?.into(),
            self.0.try_clone()?.into(),
            self.0.try_clone()?.into(),
        ))
    }

    pub fn session_leader(&self) -> impl FnMut() -> std::io::Result<()> {
        let pts_fd = self.0.as_raw_fd();
        move || {
            rustix::process::setsid()?;
            rustix::process::ioctl_tiocsctty(unsafe {
                std::os::fd::BorrowedFd::borrow_raw(pts_fd)
            })?;
            Ok(())
        }
    }
}

impl From<Pts> for std::os::fd::OwnedFd {
    fn from(pts: Pts) -> Self {
        pts.0
    }
}

impl std::os::fd::AsFd for Pts {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl std::os::fd::AsRawFd for Pts {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }
}
