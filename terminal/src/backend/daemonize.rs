use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::num::NonZeroI32;
use std::os::fd::IntoRawFd as _;

use super::cli::Cli;

pub fn daemonize(cli: Cli) -> std::io::Result<()> {
    if let Some(pid) = cli.read_pid()? {
        return Err(std::io::Error::new(
            ErrorKind::AddrInUse,
            format!("Already running PID = {pid}"),
        ));
    }

    match fork()? {
        Some(_pid) => std::process::exit(0),
        None => { /* in the child process */ }
    }

    check_err(unsafe { nix::libc::setsid() }, |r| r != 1)?;

    match fork()? {
        Some(pid) => {
            println!("Server running in the background. PID = {pid}");
            cli.save_pidfile(pid)?;
            std::process::exit(0);
        }
        None => { /* in the child process */ }
    }

    redirect_to_null(nix::libc::STDOUT_FILENO)?;
    redirect_to_null(nix::libc::STDERR_FILENO)?;

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
