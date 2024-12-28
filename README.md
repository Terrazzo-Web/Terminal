# Rust setup

- `rustup update`
- `rustup toolchain install nightly`
- `cargo install cargo-watch`
- `cargo install wasm-pack` from https://rustwasm.github.io/wasm-pack/installer/
- `cargo install stylance-cli`

# Build code
- `cargo +nightly watch -c -x fmt`
- `cargo run --bin web-terminal --features server`
- `cargo run --bin web-terminal --release --features server,max_level_info` to run it
- `cargo build --bin web-terminal --release --features server,max_level_info && nohup ./target/release/web-terminal > /dev/null 2>&1 &` to run it in the background

# wasm-pack

# Clippy
- `cargo clippy --bin web-terminal --features server,max_level_debug`
- `cargo clippy --bin web-terminal --features server,max_level_info`
- `cargo clippy --features client,max_level_debug`
- `cargo clippy --features client,max_level_info`

# Icons
- https://icons.getbootstrap.com/
