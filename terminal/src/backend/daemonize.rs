use std::fs::OpenOptions;
use std::num::NonZeroI32;
use std::os::fd::IntoRawFd as _;

use nameth::NamedEnumValues as _;
use nameth::nameth;

use super::config::pidfile::ReadPidfileError;
use super::config::pidfile::SavePidfileError;
use super::config::server::ServerConfig;

pub fn daemonize(server_config: &ServerConfig) -> Result<(), DaemonizeServerError> {
    if let Some(pid) = server_config.read_pid()? {
        return Err(DaemonizeServerError::AlreadyRunning { pid });
    }

    match fork().map_err(DaemonizeServerError::Fork)? {
        Some(_pid) => std::process::exit(0),
        None => { /* in the child process */ }
    }

    check_err(unsafe { nix::libc::setsid() }, |r| r != 1).map_err(DaemonizeServerError::Setsid)?;

    match fork().map_err(DaemonizeServerError::Fork)? {
        Some(pid) => {
            server_config.save_pidfile(pid)?;
            std::process::exit(0);
        }
        None => { /* in the child process */ }
    }

    redirect_to_null(nix::libc::STDOUT_FILENO).map_err(|error| DaemonizeServerError::IO {
        io: "stdout",
        error,
    })?;
    redirect_to_null(nix::libc::STDERR_FILENO).map_err(|error| DaemonizeServerError::IO {
        io: "stderr",
        error,
    })?;

    Ok(())
}

fn fork() -> std::io::Result<Option<NonZeroI32>> {
    let pid = check_err(unsafe { nix::libc::fork() }, |pid| pid != -1)?;
    return Ok(NonZeroI32::new(pid));
}

fn redirect_to_null(fd: nix::libc::c_int) -> std::io::Result<()> {
    let null_device = if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    };

    let null_file = OpenOptions::new().write(true).open(null_device)?;
    let null_stdout = null_file.into_raw_fd();
    check_err(unsafe { nix::libc::dup2(null_stdout, fd) }, |r| r != -1)?;
    Ok(())
}

fn check_err<IsOk: FnOnce(R) -> bool, R: Copy>(result: R, is_ok: IsOk) -> std::io::Result<R> {
    if is_ok(result) {
        Ok(result)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum DaemonizeServerError {
    #[error("[{n}] {0}", n = self.name())]
    ReadPidfile(#[from] ReadPidfileError),

    #[error("[{n}] Already running PID = {pid}", n = self.name())]
    AlreadyRunning { pid: i32 },

    #[error("[{n}] {0}", n = self.name())]
    Fork(std::io::Error),

    #[error("[{n}] {0}", n = self.name())]
    Setsid(std::io::Error),

    #[error("[{n}] {0}", n = self.name())]
    SavePidfile(#[from] SavePidfileError),

    #[error("[{n}] failed to redirect {io}", n = self.name())]
    IO {
        io: &'static str,
        error: std::io::Error,
    },
}
