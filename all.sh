#!/bin/bash

set -e

cargo check --locked --no-default-features --features max-level-debug,diagnostics,client-all,server-all
cargo check --locked --bin terrazzo-terminal --no-default-features --features server-all --features max-level-debug,diagnostics
cargo check --locked --bin terrazzo-terminal --no-default-features --features server-all --features max-level-info
cargo check --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features terminal-server
cargo check --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features text-editor-server
cargo check --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features converter-server
cargo check --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features port-forward-server
cargo check --locked --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo check --locked --bin game --no-default-features --features server,max-level-info --release

cargo clippy --locked --no-default-features --features max-level-debug,diagnostics,client-all,server-all
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features server-all --features max-level-debug,diagnostics
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features server-all --features max-level-info
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features terminal-server
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features text-editor-server
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features converter-server
cargo clippy --locked --bin terrazzo-terminal --no-default-features --features max-level-debug,debug,diagnostics --features port-forward-server
cargo clippy --locked --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo clippy --locked --bin game --no-default-features --features server,max-level-info --release

cargo test --locked --no-default-features --features server-all,client-all

cargo build --locked --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo build --locked --bin game --no-default-features --features server,max-level-info --release
cargo build --locked --bin terrazzo-terminal --no-default-features --features server-all,max-level-debug,diagnostics
cargo build --locked --bin terrazzo-terminal --no-default-features --features server-all,max-level-info --release

cargo doc --locked --all-features
