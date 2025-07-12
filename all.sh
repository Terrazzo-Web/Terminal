#!/bin/bash

set -e

cargo check --no-default-features --features client,max-level-debug,diagnostics
cargo check --no-default-features --features client,max-level-info --release
cargo check --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo check --bin game --no-default-features --features server,max-level-info --release
cargo check --bin terrazzo-terminal --no-default-features --features server,max-level-debug,debug,diagnostics
cargo check --bin terrazzo-terminal --no-default-features --features server,max-level-info --release

cargo clippy --no-default-features --features client,max-level-debug,diagnostics
cargo clippy --no-default-features --features client,max-level-info --release
cargo clippy --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo clippy --bin game --no-default-features --features server,max-level-info --release
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max-level-debug,debug,diagnostics
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max-level-info --release

cargo test --no-default-features --features client
cargo test --no-default-features --features server
cargo test --no-default-features --features client,server

cargo build --bin game --no-default-features --features server,max-level-debug,debug,diagnostics
cargo build --bin game --no-default-features --features server,max-level-info --release
cargo build --bin terrazzo-terminal --no-default-features --features server,max-level-debug,diagnostics
cargo build --bin terrazzo-terminal --no-default-features --features server,max-level-info --release
cargo doc --all-features
