# Project Guidelines

## Read this first

Do NOT scan the entire repository unless necessary.
Start from entry points and follow references.

Prefer targeted file reads over broad exploration.

## Project overview

- `game/`: small game crate.
- `pty/`: PTY emulator and process plumbing.
- `terminal/`: main web terminal app built with Terrazzo.
- Server code in `terminal/src/backend/` + `terminal/src/server.rs`.
- Client code in `terminal/src/frontend/` and feature folders.
- Feature slices: `converter`, `portforward`, `terminal`, `text_editor`.

## Terrazzo guide

### Server-side code

- Keep server startup and backend wiring in the `terminal/src/backend/` area and the `terminal/src/server.rs` entry point.
- Use feature-gated server code with `#[cfg(feature = "server")]` when logic should only exist on the native/server build.
- Define RPC-style server functions with `#[server(...)]` close to the feature they support, then keep the server-only implementation details in adjacent `backend` modules when needed.
- Prefer following existing feature slices such as `converter`, `portforward`, `terminal`, and `text_editor` instead of creating cross-cutting server code in unrelated folders.

### Client-side code

- Keep Terrazzo UI code in client modules guarded by `#[cfg(feature = "client")]`.
- Build UI with Terrazzo templates and signals (`#[html]`, `#[template]`, `XSignal`, `XTemplate`) instead of ad hoc DOM manipulation.
- Put feature UI next to its feature module, following existing patterns like `terminal/src/converter/ui.rs` and `terminal/src/frontend/`.
- Keep styling in nearby `.scss` files and import it with `stylance::import_style!`.
- Use the shared API or server-function layer for client/server communication rather than duplicating fetch logic inline.

## Notes for AI Agents
- Use `#[cfg(feature = "server")]` for server-only logic.
- Define RPC-style `#[server(...)]` functions near feature module; backend module contains implementation.
- Prefer shared API/server-function layer over raw fetch logic.
- Ask before adding production dependencies.
- Target minimal, concise change in agent work; avoid monolithic refactors unless requested.
- Do not scan the whole repository unless necessary; start at entry points and follow references.
- Use targeted reads and avoid broad exploration.
- Link to docs rather than duplicating (e.g., `README.md`, `terminal/AGENTS.md`).
- Ask for confirmation before adding new production dependencies.
- `./all.sh` must be run after touching Rust sources.

