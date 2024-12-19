#!/bin/bash

set -e

cargo test --features=client
cargo test --features=server
cargo test --features=client,server
cargo clippy --features client,max_level_debug
cargo clippy --features client,max_level_info
cargo clippy --bin game --features server,max_level_debug
cargo clippy --bin game --features server,max_level_info
cargo clippy --bin web-terminal --features server,max_level_debug
cargo clippy --bin web-terminal --features server,max_level_info
cargo build --bin game --features server,max_level_debug
cargo build --bin game --features server,max_level_info --release
cargo build --bin web-terminal --features server,max_level_debug
cargo build --bin web-terminal --features server,max_level_info --release
