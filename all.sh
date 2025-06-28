#!/bin/bash

set -e

cargo check --no-default-features --features client,max_level_debug,diagnostics
cargo check --no-default-features --features client,max_level_info --release
cargo check --bin game --no-default-features --features server,max_level_debug,debug,diagnostics
cargo check --bin game --no-default-features --features server,max_level_info --release
cargo check --bin terrazzo-terminal --no-default-features --features server,max_level_debug,debug,diagnostics
cargo check --bin terrazzo-terminal --no-default-features --features server,max_level_info --release

cargo clippy --no-default-features --features client,max_level_debug,diagnostics
cargo clippy --no-default-features --features client,max_level_info --release
cargo clippy --bin game --no-default-features --features server,max_level_debug,debug,diagnostics
cargo clippy --bin game --no-default-features --features server,max_level_info --release
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max_level_debug,debug,diagnostics
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max_level_info --release

cargo test --no-default-features --features client
cargo test --no-default-features --features server
cargo test --no-default-features --features client,server

cargo build --bin game --no-default-features --features server,max_level_debug,debug,diagnostics
cargo build --bin game --no-default-features --features server,max_level_info --release
cargo build --bin terrazzo-terminal --no-default-features --features server,max_level_debug,diagnostics
cargo build --bin terrazzo-terminal --no-default-features --features server,max_level_info --release
cargo doc --all-features
