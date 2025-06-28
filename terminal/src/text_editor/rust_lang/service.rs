#![cfg(feature = "server")]

use std::process::Stdio;

use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;
use tokio::process::Command;

async fn run_cargo_check(base_path: &str, file_path: &str) {
    // The command to run: e.g., "cargo check --message-format=json"
    let mut child = Command::new("cargo")
        .args(["check", "--message-format=json"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo process");

    let stdout = child.stdout.take().expect("Failed to capture stdout");

    let mut reader = BufReader::new(stdout).lines();

    // Read the output line by line
    while let line_result = reader.next_line().await {
        match line_result {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(&line) {
                    Ok(json_value) => {
                        // You can filter messages here if needed, e.g.:
                        // if json_value.get("reason") != Some(&Value::String("compiler-artifact".into())) {
                        println!("{}", json_value);
                        // }
                    }
                    Err(e) => {
                        eprintln!("Invalid JSON: {}\nLine: {}", e, line);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }

    // Optional: Wait for the child process to finish
    let status = child.wait().expect("Failed to wait on child");
    if !status.success() {
        eprintln!("cargo exited with status: {}", status);
    }
}

/// https://github.com/rust-lang/cargo/blob/rust-1.87.0/src/cargo/util/machine_message.rs#L23
struct CargoCheckMessage {
    reason: String,
    package_id: String,
    manifest_path: String,
    target: TargetInfo,
    message: CompilerMessage,
}

struct TargetInfo {
    kind: Vec<String>,
    crate_types: Vec<String>,
    name: String,
    src_path: String,
    edition: String,
    doc: bool,
    doctest: bool,
    test: bool,
}

struct CompilerMessage {}

/*
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
*/
