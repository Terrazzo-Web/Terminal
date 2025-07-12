# Rust setup

- `rustup update`
- `rustup toolchain install nightly` to use `cargo +nightly fmt`
- `cargo install cargo-watch`
- `cargo install wasm-pack` from https://rustwasm.github.io/wasm-pack/installer/
- `cargo install stylance-cli`

# Build code
- `cargo +nightly watch -c -x fmt`
- `cargo run --bin terrazzo-terminal --no-default-features --features server`
- `cargo run --bin terrazzo-terminal --release --no-default-features --features server,max-level-info` to run it
- `cargo build --bin terrazzo-terminal --release --no-default-features --features server,max-level-info && nohup ./target/release/terrazzo-terminal > /dev/null 2>&1 &` to run it in the background

# Clippy
- `cargo clippy --bin terrazzo-terminal --no-default-features --features server,max-level-debug`
- `cargo clippy --bin terrazzo-terminal --no-default-features --features server,max-level-info`
- `cargo clippy --no-default-features --features client,max-level-debug`
- `cargo clippy --no-default-features --features client,max-level-info`

# Icons
- https://icons.getbootstrap.com/
