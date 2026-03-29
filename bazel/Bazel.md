# Bazel plan for `game`

This note is only about the `game/` crate. It deliberately ignores `terminal/`.

## What Cargo does today

`game/build.rs` runs only for the server build path:

- it skips work unless the `server` feature is enabled
- it calls `terrazzo_build::build(...)`
- that helper runs `wasm-pack build --target web` for the same crate with the `client` feature enabled
- it moves the generated wasm bundle into `game/target/assets/wasm/`
- it runs `stylance` and writes CSS to `game/target/css/game.scss`

The server then embeds those generated files at compile time from:

- `game/target/assets/wasm/game.js`
- `game/target/assets/wasm/game_bg.wasm`
- `game/target/css/game.scss`

Relevant files:

- `game/build.rs`
- `game/src/assets.rs`
- `game/assets/bootstrap.js`
- `game/Cargo.toml`

## Why this does not map cleanly to Bazel as-is

The main issue is not just "`build.rs` calls `wasm-pack`".

`game/src/assets.rs` uses Terrazzo macros that compile generated assets into the server binary with `include_bytes!` / `include_scss!`. Those macros resolve paths relative to `CARGO_MANIFEST_DIR`.

That means Bazel must make the generated files exist before the Rust server target is compiled.

In other words, a Bazel `data = [...]` edge is not enough. The generated wasm and CSS files are compile-time inputs.

## Recommended Bazel approach

### 0. Add Bazel module setup first

Before any build targets can exist, the repo needs Bazel module setup at the workspace root.

At minimum:

- a root `MODULE.bazel`
- Bazel rules for Rust
- Bazel rules or repository setup for any non-Rust tools invoked during the build

For Rust, the expected starting point is:

- [`rules_rust`](https://bazelbuild.github.io/rules_rust/)

That is the ruleset that should own:

- Rust library targets for `game`
- the Rust binary target for `game/src/server.rs`
- Cargo dependency translation / crate universe setup
- Rust compile-time environment variables used to point at Bazel-generated assets

In practice, the first Bazel bring-up should decide and document:

- whether dependencies come from a Cargo lock import via `crate_universe`
- how `wasm-pack` is provided to Bazel
- how `stylance` is provided to Bazel

Without `MODULE.bazel` plus `rules_rust`, the rest of this plan cannot be wired up.

### 1. Disable the Cargo-side wasm build for Bazel

Build the server crate with the `no_wasm_build` feature enabled, in addition to `server`.

That avoids the `build.rs` call to `wasm-pack` and lets Bazel own asset generation.

Target feature set for the server build:

- `server`
- `no_wasm_build`

Target feature set for the wasm/client build:

- `client`

Optional passthrough features:

- `max-level-info`
- `max-level-debug`
- `debug`

### 2. Have Bazel build the wasm bundle explicitly

Create a Bazel rule that runs roughly:

```bash
wasm-pack build \
  --target web \
  --no-default-features \
  --features client \
  --target-dir target/wasm
```

Expected outputs:

- `pkg/game.js`
- `pkg/game_bg.wasm`

These are the files imported by `game/assets/bootstrap.js`.

### 3. Have Bazel build the Stylance CSS explicitly

Create a Bazel rule that runs:

```bash
stylance .
```

from the `game/` directory.

Expected output:

- `target/css/game.scss`

Even though the filename ends in `.scss`, the Terrazzo server macro serves it as CSS.

### 4. Make the generated files visible at Rust compile time

This is the critical part.

There are two possible designs.

#### Preferred: small Rust refactor

Make `game/src/assets.rs` read the generated file paths from Bazel-provided `rustc_env` values instead of hard-coding `target/...` under `CARGO_MANIFEST_DIR`.

For example, conceptually:

- `GAME_CSS_ASSET`
- `GAME_WASM_JS_ASSET`
- `GAME_WASM_WASM_ASSET`

Then Bazel can:

- generate the files anywhere in the output tree
- pass their paths to the Rust compile action
- let the server crate `include_bytes!` them directly

This is the cleanest Bazel shape because it avoids trying to materialize generated files inside the source tree.

#### Possible but awkward: synthetic crate root

Make a Bazel action build a synthetic directory that looks like:

```text
game/
  src/...
  assets/...
  target/css/game.scss
  target/assets/wasm/game.js
  target/assets/wasm/game_bg.wasm
```

and compile the server crate with that directory acting as `CARGO_MANIFEST_DIR`.

This keeps the Rust code unchanged, but it is much less pleasant than the small refactor above.

## Suggested Bazel target graph

At a high level:

```text
game_client_wasm
game_css
game_static_assets
game_server_assets
game_server_lib
game_server_bin
```

Where:

- `game_client_wasm` runs `wasm-pack`
- `game_css` runs `stylance`
- `game_static_assets` exposes `game/assets/**`
- `game_server_assets` combines static assets plus generated wasm/CSS
- `game_server_lib` builds Rust with features `server,no_wasm_build`
- `game_server_bin` is the executable from `game/src/server.rs`

## What the server ultimately needs

For the server binary to work, Bazel must provide these logical assets:

- `/assets/index.html`
- `/assets/bootstrap.js`
- `/assets/images/favicon.ico`
- `/assets/game/**`
- generated CSS served as `game.css`
- generated wasm files served as:
  - `wasm/game.js`
  - `wasm/game_bg.wasm`

`bootstrap.js` imports `./wasm/game.js`, so preserving that output name matters.

## Minimal source change I would make

If we want this to work cleanly in Bazel, I would change only one thing in Rust:

- stop hard-coding `target/css/game.scss` and `target/assets/wasm/...` in `game/src/assets.rs`
- replace them with compile-time environment variables supplied by Bazel

Everything else can stay conceptually the same:

- Bazel generates wasm
- Bazel generates CSS
- the server still embeds those assets and serves them through the existing Terrazzo code

## Practical next step

The first implementation pass should be:

1. Add Bazel wasm and CSS generation rules.
2. Patch `game/src/assets.rs` to accept Bazel-provided asset paths.
3. Build the server crate with `server,no_wasm_build`.
4. Verify that the generated server still serves:
   - `/`
   - `/static/bootstrap.js`
   - `/static/wasm/game.js`
   - `/static/wasm/game_bg.wasm`
   - `/static/game.css`

## Validation note

I did not validate Bazel commands end-to-end in this repo because there is no Bazel setup checked in yet, and local `./ubuntu.sh bazel ...` currently fails before startup with a rootless Podman `newuidmap` error on this machine.
