#![cfg(feature = "server")]
#![allow(unused)]

use std::path::Path;
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

async fn run_cargo_check(
    base_path: impl AsRef<Path>,
    features: &[&str],
) -> Result<(), CargoCheckError> {
    let mut reader: tokio::io::Lines<BufReader<tokio::process::ChildStdout>> = {
        let mut command = Command::new("cargo");
        command
            .current_dir(base_path)
            .args(["check", "--message-format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if !features.is_empty() {
            command.arg("--features").arg(features.join(","));
        }
        let mut child = command.spawn().map_err(CargoCheckError::SpawnProcess)?;

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
        if message.reason != "compiler-message" {
            continue;
        }

        dbg!(message);
        results.push(());
    }
    drop(results);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    const RUST_LANG_CHECKS: &'static str = "src/text_editor/rust_lang/tests/rust_lang_checks";

    #[tokio::test]
    async fn some_unused_method() {
        enable_tracing_for_tests();
        let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RUST_LANG_CHECKS);
        let () = super::run_cargo_check(base_path, &["some_unused_method"])
            .await
            .unwrap();
    }
}
