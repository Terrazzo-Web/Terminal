use std::fs::File;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;

use nameth::NamedType as _;
use nameth::nameth;

use super::server::ServerConfig;

impl ServerConfig {
    pub fn read_pid(&self) -> Result<Option<i32>, ReadPidfileError> {
        if !self.pid_filepath().exists() {
            return Ok(None);
        }

        let mut pid_file = File::open(&self.pidfile)?;
        let mut pid_string = String::default();
        pid_file.read_to_string(&mut pid_string)?;
        Ok(pid_string.parse().map(Some).map_err(|error| {
            std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse pid '{pid_string}': {error}"),
            )
        })?)
    }

    pub fn save_pidfile(&self, pid: std::num::NonZero<i32>) -> Result<(), SavePidfileError> {
        let terrazzo_config_dir = self.pid_filepath().parent().ok_or_else(|| {
            std::io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Failed to get pidfile parent folder: {:?}",
                    self.pid_filepath()
                ),
            )
        })?;
        std::fs::create_dir_all(terrazzo_config_dir)?;
        let mut pid_file = File::create(&self.pidfile)?;
        pid_file.write_all(pid.get().to_string().as_bytes())?;
        Ok(())
    }

    pub fn delete_pidfile(&self) -> Result<(), DeletePidfileError> {
        Ok(std::fs::remove_file(&self.pidfile)?)
    }

    fn pid_filepath(&self) -> &std::path::Path {
        std::path::Path::new(&self.pidfile)
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct ReadPidfileError(#[from] std::io::Error);

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct SavePidfileError(#[from] std::io::Error);

#[nameth]
#[derive(thiserror::Error, Debug)]
#[error("[{n}] {0}", n = Self::type_name())]
pub struct DeletePidfileError(#[from] std::io::Error);
