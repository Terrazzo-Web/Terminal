#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnostic {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "bp"))]
    pub base_path: String,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "fp"))]
    pub file_path: String,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "l"))]
    pub level: String,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "m"))]
    pub message: String,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub code: Option<SyntheticDiagnosticCode>,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "s"))]
    pub spans: Vec<SyntheticDiagnosticSpan>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnosticCode {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    pub code: String,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "e"))]
    pub explanation: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SyntheticDiagnosticSpan {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file_name: String,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "bs"))]
    pub byte_start: u32,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "be"))]
    pub byte_end: u32,

    /// 1-based.
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "ls"))]
    pub line_start: u32,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "le"))]
    pub line_end: u32,

    /// 1-based.
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "cs"))]
    pub column_start: u32,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "ce"))]
    pub column_end: u32,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "sr"))]
    pub suggested_replacement: Option<String>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "sa"))]
    pub suggestion_applicability: Option<Applicability>,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L58
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Applicability {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "a"))]
    MachineApplicable,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    MaybeIncorrect,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "c"))]
    HasPlaceholders,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "d"))]
    Unspecified,
}

#[cfg(feature = "server")]
mod convert {
    use std::borrow::Cow;
    use std::path::Path;

    use super::super::messages::CargoCheckMessage;
    use super::super::messages::Diagnostic;
    use super::SyntheticDiagnostic;
    use super::SyntheticDiagnosticCode;
    use super::SyntheticDiagnosticSpan;

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
                        suggested_replacement: span
                            .suggested_replacement
                            .as_ref()
                            .map(Cow::to_string),
                        suggestion_applicability: span.suggestion_applicability,
                    })
                    .collect(),
            });
            for child in &diagnostic.children {
                Self::all(base_path, file_path, child, result);
            }
        }
    }
}
