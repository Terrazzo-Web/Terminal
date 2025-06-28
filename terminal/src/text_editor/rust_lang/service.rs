#![cfg(feature = "server")]
#![expect(unused)]

use std::process::Stdio;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;
use tokio::process::Command;
use tracing::debug;

#[nameth]
#[derive(Debug, thiserror::Error)]
pub enum CargoCheckError {
    #[error("[{n}] {0}", n = self.name())]
    SpawnProcess(std::io::Error),

    #[error("[{n}] Process doesn't have an stdout", n = self.name())]
    MissingStdout,

    #[error("[{n}] {0}", n = self.name())]
    Failure(std::io::Error),
}

async fn run_cargo_check(base_path: &str, file_path: &str) -> Result<(), CargoCheckError> {
    let mut reader: tokio::io::Lines<BufReader<tokio::process::ChildStdout>> = {
        let mut child = Command::new("cargo")
            .args(["check", "--message-format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(CargoCheckError::SpawnProcess)?;

        let stdout = child.stdout.take().ok_or(CargoCheckError::MissingStdout)?;
        BufReader::new(stdout).lines()
    };

    let mut results = vec![];
    loop {
        let next_line = reader.next_line().await;
        let next_line = match next_line {
            Ok(Some(next_line)) => next_line,
            Ok(None) => break,
            Err(error) => {
                if results.is_empty() {
                    return Err(CargoCheckError::Failure(error));
                } else {
                    debug!("Bad line: {error}");
                    break;
                }
            }
        };
        let next_line = next_line.trim();
        if next_line.is_empty() {
            continue;
        }

        let Ok(message) = serde_json::from_str::<super::messages::CargoCheckMessage>(next_line)
            .inspect_err(|error| debug!("Invalid cargo check JSON: {error}"))
        else {
            continue;
        };
        drop(message);
        results.push(());
    }
    drop(results);
    Ok(())
}
