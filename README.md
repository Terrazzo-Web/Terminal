# Rust setup

- `rustup update`
- `rustup toolchain install nightly`
- `cargo install cargo-watch`

# Build code
- `cargo +nightly watch -c -x fmt`
- `cargo run --bin terrazzo-server --features server`
- `cargo run --bin terrazzo-server --release --features server,max_level_info` to run it
- `cargo build --bin terrazzo-server --release --features server,max_level_info && nohup ./target/release/terrazzo-server >/dev/null 2>&1 &` to run it in the background

# wasm-pack
- `cargo install wasm-pack` from https://rustwasm.github.io/wasm-pack/installer/

# Stylance
- `cargo install stylance-cli`
- `stylance --watch .`

# Clippy
- `cargo clippy --bin terrazzo-server --features server,max_level_debug`
- `cargo clippy --bin terrazzo-server --features server,max_level_info`
- `cargo clippy --features client,max_level_debug`
- `cargo clippy --features client,max_level_info`

# Sass
- `npm install sass`
- `npm exec sass -- --watch --no-source-map target/css/generated.scss assets/css/generated.cs`

# Icons
- https://icons.getbootstrap.com/
