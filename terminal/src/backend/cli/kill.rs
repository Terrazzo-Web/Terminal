use nameth::NamedEnumValues as _;
use nameth::nameth;
use nix::errno::Errno;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::Cli;
use super::pidfile::DeletePidfileError;
use super::pidfile::ReadPidfileError;

impl Cli {
    pub fn kill(&self) -> Result<(), KillServerError> {
        let pid = self
            .read_pid()?
            .ok_or_else(|| KillServerError::PidfileNotFound {
                pidfile: self.pidfile.to_owned(),
            })?;

        let result = kill_aux(pid);
        self.delete_pidfile()?;
        return result;
    }
}

fn kill_aux(pid: i32) -> Result<(), KillServerError> {
    signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
        .map_err(|errno| KillServerError::KillError { pid, errno })
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum KillServerError {
    #[error("[{n}] {0}", n = self.name())]
    ReadPidfile(#[from] ReadPidfileError),

    #[error("[{n}] Pid file '{pidfile}' not found", n = self.name())]
    PidfileNotFound { pidfile: String },

    #[error("[{n}] {0}", n = self.name())]
    DeletePidfile(#[from] DeletePidfileError),

    #[error("[{n}] Failed to kill process {pid}: {errno}", n = self.name())]
    KillError { pid: i32, errno: Errno },
}
