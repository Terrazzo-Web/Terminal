#!/bin/bash

set -e

cargo test --no-default-features --features client
cargo test --no-default-features --features server
cargo test --no-default-features --features client,server
cargo clippy --no-default-features --features client,max_level_debug,diagnostics
cargo clippy --no-default-features --features client,max_level_info
cargo clippy --bin game --no-default-features --features server,max_level_debug,diagnostics
cargo clippy --bin game --no-default-features --features server,max_level_info
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max_level_debug,diagnostics
cargo clippy --bin terrazzo-terminal --no-default-features --features server,max_level_info
cargo build --bin game --no-default-features --features server,max_level_debug,diagnostics
cargo build --bin game --no-default-features --features server,max_level_info --release
cargo build --bin terrazzo-terminal --no-default-features --features server,max_level_debug,diagnostics
cargo build --bin terrazzo-terminal --no-default-features --features server,max_level_info --release
cargo doc --all-features
