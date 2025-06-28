#![cfg(feature = "server")]
#![allow(unused)]

pub use std::borrow::Cow;

use super::synthetic::Applicability;

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/src/cargo/util/machine_message.rs#L23
#[derive(Debug, serde::Deserialize)]
pub struct CargoCheckMessage<'a> {
    #[serde(borrow)]
    pub reason: Cow<'a, str>,

    #[cfg(debug_assertions)]
    #[serde(borrow)]
    pub package_id: Cow<'a, str>,

    #[serde(borrow)]
    pub manifest_path: Cow<'a, str>,

    #[cfg(debug_assertions)]
    #[serde(borrow)]
    pub target: TargetInfo<'a>,

    #[serde(borrow)]
    pub message: Diagnostic<'a>,
}

#[cfg(debug_assertions)]
#[derive(Debug, serde::Deserialize)]
pub struct TargetInfo<'a> {
    #[serde(borrow)]
    pub kind: Vec<Cow<'a, str>>,

    #[serde(borrow)]
    pub crate_types: Vec<Cow<'a, str>>,

    #[serde(borrow)]
    pub name: Cow<'a, str>,

    #[serde(borrow)]
    pub src_path: Cow<'a, str>,

    #[serde(borrow)]
    pub edition: Cow<'a, str>,

    pub doc: bool,
    pub doctest: bool,
    pub test: bool,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L11
#[derive(Debug, serde::Deserialize)]
pub struct Diagnostic<'a> {
    #[serde(borrow)]
    pub message: Cow<'a, str>,

    #[serde(borrow)]
    pub code: Option<DiagnosticCode<'a>>,

    #[serde(borrow)]
    pub level: Cow<'a, str>,

    #[serde(borrow)]
    pub spans: Vec<DiagnosticSpan<'a>>,

    #[serde(borrow)]
    pub children: Vec<Diagnostic<'a>>,

    #[cfg(debug_assertions)]
    #[serde(borrow)]
    pub rendered: Option<Cow<'a, str>>,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L110
#[derive(Debug, serde::Deserialize)]
pub struct DiagnosticCode<'a> {
    #[serde(borrow)]
    pub code: Cow<'a, str>,

    #[serde(borrow)]
    pub explanation: Option<Cow<'a, str>>,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L26
#[derive(Debug, serde::Deserialize)]
pub struct DiagnosticSpan<'a> {
    #[serde(borrow)]
    pub file_name: Cow<'a, str>,

    pub byte_start: u32,
    pub byte_end: u32,

    /// 1-based.
    pub line_start: u32,
    pub line_end: u32,

    /// 1-based.
    pub column_start: u32,
    pub column_end: u32,

    /// The point where the error actually occurred.
    pub is_primary: bool,

    #[cfg(debug_assertions)]
    #[serde(borrow)]
    pub text: Vec<DiagnosticSpanLine<'a>>,

    #[serde(borrow)]
    pub label: Option<Cow<'a, str>>,

    #[serde(borrow)]
    pub suggested_replacement: Option<Cow<'a, str>>,
    pub suggestion_applicability: Option<Applicability>,

    /// Macro invocations that created the code at this span, if any.
    #[serde(borrow)]
    pub expansion: Option<Box<DiagnosticSpanMacroExpansion<'a>>>,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L82
#[derive(Debug, serde::Deserialize)]
pub struct DiagnosticSpanLine<'a> {
    #[serde(borrow)]
    pub text: Cow<'a, str>,
    pub highlight_start: u32,
    pub highlight_end: u32,
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/crates/rustfix/src/diagnostics.rs#L93
#[derive(Debug, serde::Deserialize)]
pub struct DiagnosticSpanMacroExpansion<'a> {
    #[serde(borrow)]
    pub span: DiagnosticSpan<'a>,

    #[serde(borrow)]
    pub macro_decl_name: Cow<'a, str>,

    #[serde(borrow)]
    pub def_site_span: Option<DiagnosticSpan<'a>>,
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    #[test]
    fn deserialize() {
        let message: super::CargoCheckMessage = serde_json::from_str(COMPILER_MESSAGE).unwrap();
        assert!(matches!(message.reason, Cow::Borrowed(_)));
    }

    const COMPILER_MESSAGE: &'static str = r#"
{
  "reason": "compiler-message",
  "package_id": "path+file:///home/richard/Documents/Terminal/terminal#terrazzo-terminal@0.1.15",
  "manifest_path": "/home/richard/Documents/Terminal/terminal/Cargo.toml",
  "target": {
    "kind": [
      "cdylib",
      "rlib"
    ],
    "crate_types": [
      "cdylib",
      "rlib"
    ],
    "name": "terrazzo_terminal",
    "src_path": "/home/richard/Documents/Terminal/terminal/src/lib.rs",
    "edition": "2024",
    "doc": true,
    "doctest": true,
    "test": true
  },
  "message": {
    "rendered": "error[E0599]: no method named `expect` found for opaque type `impl futures::Future<Output = Result<ExitStatus, std::io::Error>>` in the current scope\n   --> terminal/src/text_editor/rust_lang/service.rs:48:31\n    |\n48  |     let status = child.wait().expect(\"Failed to wait on child\");\n    |                               ^^^^^^\n    |\nhelp: there is a method `explicit` with a similar name, but with different arguments\n   --> /home/richard/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/asn1-rs-0.7.1/src/traits.rs:324:5\n    |\n324 |     fn explicit(self, class: Class, tag: u32) -> TaggedParser<'a, Explicit, Self, E> {\n    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\nhelp: consider `await`ing on the `Future` and calling the method on its `Output`\n    |\n48  |     let status = child.wait().await.expect(\"Failed to wait on child\");\n    |                               ++++++\n\n",
    "$message_type": "diagnostic",
    "children": [
      {
        "children": [],
        "code": null,
        "level": "help",
        "message": "there is a method `explicit` with a similar name, but with different arguments",
        "rendered": null,
        "spans": [
          {
            "byte_end": 9990,
            "byte_start": 9910,
            "column_end": 85,
            "column_start": 5,
            "expansion": null,
            "file_name": "/home/richard/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/asn1-rs-0.7.1/src/traits.rs",
            "is_primary": true,
            "label": null,
            "line_end": 324,
            "line_start": 324,
            "suggested_replacement": null,
            "suggestion_applicability": null,
            "text": [
              {
                "highlight_end": 85,
                "highlight_start": 5,
                "text": "    fn explicit(self, class: Class, tag: u32) -> TaggedParser<'a, Explicit, Self, E> {"
              }
            ]
          }
        ]
      },
      {
        "children": [],
        "code": null,
        "level": "help",
        "message": "consider `await`ing on the `Future` and calling the method on its `Output`",
        "rendered": null,
        "spans": [
          {
            "byte_end": 1577,
            "byte_start": 1577,
            "column_end": 31,
            "column_start": 31,
            "expansion": null,
            "file_name": "terminal/src/text_editor/rust_lang/service.rs",
            "is_primary": true,
            "label": null,
            "line_end": 48,
            "line_start": 48,
            "suggested_replacement": "await.",
            "suggestion_applicability": "MaybeIncorrect",
            "text": [
              {
                "highlight_end": 31,
                "highlight_start": 31,
                "text": "    let status = child.wait().expect(\"Failed to wait on child\");"
              }
            ]
          }
        ]
      }
    ],
    "code": {
      "code": "E0599",
      "explanation": "This error occurs when a method is used on a type which doesn't implement it:\n\nErroneous code example:\n\n```compile_fail,E0599\nstruct Mouth;\n\nlet x = Mouth;\nx.chocolate(); // error: no method named `chocolate` found for type `Mouth`\n               //        in the current scope\n```\n\nIn this case, you need to implement the `chocolate` method to fix the error:\n\n```\nstruct Mouth;\n\nimpl Mouth {\n    fn chocolate(&self) { // We implement the `chocolate` method here.\n        println!(\"Hmmm! I love chocolate!\");\n    }\n}\n\nlet x = Mouth;\nx.chocolate(); // ok!\n```\n"
    },
    "level": "error",
    "message": "no method named `expect` found for opaque type `impl futures::Future<Output = Result<ExitStatus, std::io::Error>>` in the current scope",
    "spans": [
      {
        "byte_end": 1583,
        "byte_start": 1577,
        "column_end": 37,
        "column_start": 31,
        "expansion": null,
        "file_name": "terminal/src/text_editor/rust_lang/service.rs",
        "is_primary": true,
        "label": null,
        "line_end": 48,
        "line_start": 48,
        "suggested_replacement": null,
        "suggestion_applicability": null,
        "text": [
          {
            "highlight_end": 37,
            "highlight_start": 31,
            "text": "    let status = child.wait().expect(\"Failed to wait on child\");"
          }
        ]
      }
    ]
  }
}
"#;
}
