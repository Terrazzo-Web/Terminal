# Terrazzo terminal

Terrazzo terminal is a simple web-based terminal built in Rust and Web Assembly 
using the [Terrazzo](https://docs.rs/terrazzo) library.

## Getting started
Pre-requisite:
- [`wasm-pack` CLI](https://rustwasm.github.io/wasm-pack/installer/)
- [`stylance-cli` CLI](https://github.com/basro/stylance-rs?tab=readme-ov-file#stylance-cli)

```
cargo install wasm-pack
cargo install stylance-cli
```

Then run `terrazzo-terminal` using
```
cargo run --bin terrazzo-terminal --release --no-default-features --features server,max_level_info
```