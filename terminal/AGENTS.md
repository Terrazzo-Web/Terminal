# AGENTS.md

## Read this first

Do not scan all of `terminal/src` at once unless the task really needs it.
Start from the feature entry point you are changing, then follow the local references.

Good entry points:

- `terminal/src/converter/mod.rs`
- `terminal/src/terminal/mod.rs`
- `terminal/src/portforward/mod.rs`
- `terminal/src/text_editor/mod.rs`

## Feature overview

### `converter`

What it does:

- Takes a block of text and derives useful alternate representations from it.
- Common outputs include parsed JWT, Base64, JSON or YAML formatting, TLS or X509 details, DNS output, timestamps, and similar developer-focused conversions.

How it works:

- The client UI lives in `terminal/src/converter/ui.rs`.
- It uses Terrazzo signals to store the current input and the list of conversions.
- Input changes are debounced in the browser, then sent through the `get_conversions` server function in `terminal/src/converter/api.rs`.
- Server-side conversion logic lives in `terminal/src/converter/service.rs`.
- The service tries recognizers in sequence and appends matching conversion results.
- The feature also persists the current input through `content_state`, so switching around the app does not immediately lose the draft.

### `terminal`

What it does:

- Provides the browser-based terminal experience backed by PTY processes on the server.
- Supports multiple tabs, tab titles, resize handling, and streaming terminal I/O between browser and backend.

How it works:

- The client tab UI starts in `terminal/src/terminal/mod.rs`.
- Each tab is modeled by `terminal/src/terminal/terminal_tab.rs`.
- XtermJS attachment and event wiring live in `terminal/src/terminal/attach.rs` and `terminal/src/terminal/javascript.rs`.
- Browser input is buffered and sent with the client terminal API under `terminal/src/api/client/terminal_api/`.
- Output streaming is coordinated by `terminal/src/api/client/terminal_api/stream.rs`, including reconnect behavior when the pipe drops.
- Server-side process state lives under `terminal/src/processes/`.
- The backend keeps a global map of active terminal definitions and PTY handles, then exposes operations such as list, write, resize, close, and stream registration.

### `portforward`

What it does:

- Lets the user define port-forward rules for a selected remote.
- Tracks whether each rule is active, pending, offline, or failed.

How it works:

- The client UI is in `terminal/src/portforward/ui.rs`.
- The UI is driven by `Manager` in `terminal/src/portforward/manager.rs`, which owns the current list and synchronizes edits.
- Changes are optimistic locally, then written back through server functions in `terminal/src/portforward/state.rs`.
- The server implementation in that same module keeps the persisted rule set and triggers the engine when rules change.
- Execution logic lives in `terminal/src/portforward/engine.rs` and related helpers.
- `schema.rs` defines the editable rule model and runtime status values.
- `sync_state.rs` tracks per-field save state so the UI can show loading and delete feedback.

### `text_editor`

What it does:

- Provides a browser-based file editor and folder browser for a selected remote.
- Supports opening files, browsing folders, search, side-view history, synchronization state, and file watching.

How it works:

- The main UI entry point is `terminal/src/text_editor/ui.rs`.
- `TextEditorManager` in `terminal/src/text_editor/manager.rs` coordinates selected paths, editor state, side view, search state, and synchronization status.
- File and folder loading flows through `fsio`, while the editor rendering is split across modules like `editor`, `folder`, `path_selector`, and `search`.
- The feature persists UI state such as base path, file path, side view, and search query using the `state` module.
- Notifications are handled through `text_editor/notify`, which watches loaded files and updates the side view when files are deleted or error out.
- CodeMirror integration lives in `code_mirror.rs` and `code_mirror.js`.
- Rust-specific editor helpers live in `rust_lang.rs` and related submodules.

## Terrazzo patterns in this crate

- Client feature UIs are usually in each feature's `ui.rs` and are guarded by `#[cfg(feature = "client")]`.
- Server entry points are often `#[server(...)]` functions near the feature code, with server-only implementation behind `#[cfg(feature = "server")]`.
- Styling stays next to the feature in `.scss` files and is imported with `stylance::import_style!`.
- Signals and templates are the default state-management and rendering pattern; follow existing `XSignal`, `XTemplate`, `#[html]`, and `#[template]` usage before introducing a different approach.
