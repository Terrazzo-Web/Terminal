use std::fs::File;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;

use clap::Parser;
use clap::ValueEnum;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use scopeguard::defer;

use super::HOST;
use super::PORT;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Whether to start or stop the terrazzo-terminal daemon.
    #[arg(long, value_enum, default_value_t = Action::Run)]
    pub action: Action,

    /// The TCP host to listen to.
    #[arg(long, default_value_t = HOST.to_owned())]
    pub host: String,

    /// The TCP port to listen to.
    #[arg(long, default_value_t = PORT)]
    pub port: u16,

    /// The file to store the pid of the daemon while it is running
    #[arg(long, default_value_t = format!("{}/.terrazzo/terminal-$port.pid", std::env::var("HOME").expect("HOME")))]
    pub pidfile: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Action {
    Run,
    Start,
    Stop,
}

impl Cli {
    pub fn kill(&self) -> std::io::Result<()> {
        let pid = self.read_pid()?.ok_or_else(|| {
            std::io::Error::new(
                ErrorKind::NotFound,
                format!("Pid file '{}' not found", self.pidfile),
            )
        })?;

        defer!(self.delete_pidfile());
        if let Err(errno) = signal::kill(Pid::from_raw(pid), Signal::SIGTERM) {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                format!("Failed to kill process {}: {}", pid, errno),
            ));
        }

        Ok(())
    }

    pub fn read_pid(&self) -> std::io::Result<Option<i32>> {
        if !self.pid_filepath().exists() {
            return Ok(None);
        }

        let mut pid_file = File::open(&self.pidfile)?;
        let mut pid_string = String::default();
        pid_file.read_to_string(&mut pid_string)?;
        Ok(pid_string.parse().ok())
    }

    pub fn save_pidfile(&self, pid: std::num::NonZero<i32>) -> std::io::Result<()> {
        let terrazzo_config_dir = self.pid_filepath().parent().expect("terrazzo config dir");
        std::fs::create_dir_all(terrazzo_config_dir)?;
        let mut pid_file = File::create(&self.pidfile)?;
        pid_file.write_all(pid.get().to_string().as_bytes())?;
        Ok(())
    }

    pub fn delete_pidfile(&self) {
        std::fs::remove_file(&self.pidfile).expect("Unlink pidfile")
    }

    pub fn pid_filepath(&self) -> &std::path::Path {
        std::path::Path::new(&self.pidfile)
    }
}
