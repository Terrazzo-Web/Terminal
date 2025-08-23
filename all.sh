#!/bin/bash

set -e

cargo check --no-default-features --features max-level-debug,diagnostics,client-all,server-all
cargo check --bin terrazzo-terminal --no-default-features --features server-all --features max-level-debug,diagnostics
cargo check --bin terrazzo-terminal --no-default-features --features server-all --features max-level-info
cargo check --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features terminal-server
cargo check --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features text-editor-server
cargo check --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features converter-server
cargo check --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features port-forward-server
cargo check --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo check --bin game --no-default-features --features server,max-level-info --release

cargo clippy --no-default-features --features max-level-debug,diagnostics,client-all,server-all
cargo clippy --bin terrazzo-terminal --no-default-features --features server-all --features max-level-debug,diagnostics
cargo clippy --bin terrazzo-terminal --no-default-features --features server-all --features max-level-info
cargo clippy --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features terminal-server
cargo clippy --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features text-editor-server
cargo clippy --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features converter-server
cargo clippy --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features port-forward-server
cargo clippy --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo clippy --bin game --no-default-features --features server,max-level-info --release

cargo test --no-default-features --features server-all,client-all

cargo build --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo build --bin game --no-default-features --features server,max-level-info --release
cargo build --bin terrazzo-terminal --no-default-features --features server-all,max-level-debug,diagnostics
cargo build --bin terrazzo-terminal --no-default-features --features server-all,max-level-info --release

cargo doc --all-features
