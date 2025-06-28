#![cfg(feature = "server")]

use std::path::Path;
use std::process::Stdio;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;
use tokio::process::Command;
use tonic::Code;
use tracing::debug;

use super::synthetic::SyntheticDiagnostic;
use crate::backend::client_service::grpc_error::GrpcError;
use crate::backend::client_service::grpc_error::IsGrpcError;

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

impl IsGrpcError for CargoCheckError {
    fn code(&self) -> Code {
        Code::Internal
    }
}

pub async fn cargo_check(
    base_path: impl AsRef<Path>,
    features: &[&str],
) -> Result<Vec<SyntheticDiagnostic>, GrpcError<CargoCheckError>> {
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
                    return Err(CargoCheckError::Failure(error))?;
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
            .inspect_err(|error| debug!("Invalid cargo check JSON: {error}: {next_line}"))
        else {
            continue;
        };
        if message.reason != "compiler-message" {
            continue;
        }

        results.extend(SyntheticDiagnostic::new(&message));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use super::super::synthetic::SyntheticDiagnosticCode;
    use super::super::synthetic::SyntheticDiagnosticSpan;

    const RUST_LANG_CHECKS: &'static str = "src/text_editor/rust_lang/tests/rust_lang_checks";

    #[tokio::test]
    async fn some_unused_method() {
        enable_tracing_for_tests();
        let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RUST_LANG_CHECKS);
        let result = super::cargo_check(&base_path, &["some_unused_method"])
            .await
            .unwrap();
        assert_eq!(base_path, PathBuf::from(&result[0].base_path));
        assert_eq!("warning", &result[0].level);
        assert_eq!(
            &Some(SyntheticDiagnosticCode {
                code: "dead_code".into(),
                explanation: None
            }),
            &result[0].code
        );
        assert_eq!(
            &SyntheticDiagnosticSpan {
                file_name: "src/main.rs".into(),
                byte_start: 88,
                byte_end: 106,
                line_start: 6,
                line_end: 6,
                column_start: 4,
                column_end: 22,
                suggested_replacement: None,
                suggestion_applicability: None
            },
            &result[0].spans[0]
        )
    }

    #[tokio::test]
    async fn method_does_not_exist() {
        enable_tracing_for_tests();
        let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(RUST_LANG_CHECKS);
        let result = super::cargo_check(&base_path, &["method_does_not_exist"])
            .await
            .unwrap();
        assert_eq!(base_path, PathBuf::from(&result[0].base_path));
        assert_eq!("error", &result[0].level);
        assert_eq!("E0599", result[0].code.as_ref().unwrap().code);
        assert!(result[0].message.contains("no method named `unwrap2`"));
    }
}
