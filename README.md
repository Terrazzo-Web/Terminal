# Rust setup

- `rustup update`
- `rustup toolchain install nightly`
- `cargo install cargo-watch`

# Build code
- `cargo +nightly watch -c -x fmt`
- `cargo run --bin web-terminal --features server`
- `cargo run --bin web-terminal --release --features server,max_level_info` to run it
- `cargo build --bin web-terminal --release --features server,max_level_info && nohup ./target/release/web-terminal > /dev/null 2>&1 &` to run it in the background

# wasm-pack
- `cargo install wasm-pack` from https://rustwasm.github.io/wasm-pack/installer/

# Stylance
- `cargo install stylance-cli`
- `stylance --watch .`

# Clippy
- `cargo clippy --bin web-terminal --features server,max_level_debug`
- `cargo clippy --bin web-terminal --features server,max_level_info`
- `cargo clippy --features client,max_level_debug`
- `cargo clippy --features client,max_level_info`

# Sass
- `npm install sass`
- `npm exec sass -- --watch --no-source-map target/css/generated.scss assets/css/generated.cs`

# Icons
- https://icons.getbootstrap.com/

# ACT
- https://nektosact.com/
- `brew install act`
- `act push --matrix os:macos-latest -P macos-latest=-self-hosted`
- TODO: Cache. See https://github.com/NoahHsu/github-act-cache-server/blob/main/src/index.js
