#![allow(unused)]

use std::borrow::Cow;
use std::path::Path;

use super::messages::CargoCheckMessage;
use crate::text_editor::rust_lang::messages::Diagnostic;

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnostic {
    pub base_path: String,
    pub file_path: String,
    pub level: String,
    pub message: String,
    pub code: Option<SyntheticDiagnosticCode>,
    pub spans: Vec<SyntheticDiagnosticSpan>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnosticCode {
    pub code: String,
    pub explanation: Option<String>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnosticSpan {
    pub file_name: String,

    pub byte_start: u32,
    pub byte_end: u32,

    /// 1-based.
    pub line_start: u32,
    pub line_end: u32,

    /// 1-based.
    pub column_start: u32,
    pub column_end: u32,

    pub suggested_replacement: Option<String>,
    pub suggestion_applicability: Option<Applicability>,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L58
#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    HasPlaceholders,
    Unspecified,
}

impl SyntheticDiagnostic {
    pub fn new(check: &CargoCheckMessage) -> Vec<Self> {
        let mut result = vec![];
        Self::all(
            &Path::new(check.manifest_path.as_ref())
                .parent()
                .unwrap_or("/".as_ref())
                .to_string_lossy(),
            &check.target.src_path,
            &check.message,
            &mut result,
        );
        return result;
    }

    fn all(base_path: &str, file_path: &str, diagnostic: &Diagnostic, result: &mut Vec<Self>) {
        result.push(Self {
            base_path: base_path.to_owned(),
            file_path: file_path.to_owned(),
            level: diagnostic.level.to_string(),
            message: diagnostic.message.to_string(),
            code: diagnostic
                .code
                .as_ref()
                .map(|code| SyntheticDiagnosticCode {
                    code: code.code.to_string(),
                    explanation: code.explanation.as_ref().map(Cow::to_string),
                }),
            spans: diagnostic
                .spans
                .iter()
                .map(|span| SyntheticDiagnosticSpan {
                    file_name: span.file_name.to_string(),
                    byte_start: span.byte_start,
                    byte_end: span.byte_end,
                    line_start: span.line_start,
                    line_end: span.line_end,
                    column_start: span.column_start,
                    column_end: span.column_end,
                    suggested_replacement: span.suggested_replacement.as_ref().map(Cow::to_string),
                    suggestion_applicability: span.suggestion_applicability,
                })
                .collect(),
        });
        for child in &diagnostic.children {
            Self::all(base_path, file_path, child, result);
        }
    }
}
