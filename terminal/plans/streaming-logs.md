# Streaming Logs Plan

## Summary

Show server logs in the authenticated app UI by wrapping the existing app content in a container that renders:

1. the active app view on top
2. a live `<ol>/<li>` log list below

Logs should come from backend `tracing::info!`, `tracing::warn!`, and `tracing::error!` events via a `#[server]` function that uses an HTTP streaming response rather than websockets.

## Tasks

### 1. Add the UI shell around `show_app`

- Update `fn show_app(#[signal] app: App, remote: XSignal<Remote>) -> XElement` in `terminal/src/frontend/login.rs`.
- Replace the direct app-only render with an outer `<div>`.
- Render the selected app content in the top section without changing app-specific behavior.
- Render a log panel below it with an ordered list (`<ol>`) of streamed log entries.
- Keep the app-switching behavior intact for terminal, text editor, converter, and port forward.

### 2. Add client-side log state and stream consumption

- Introduce a client-side log item type with enough data to render and expire entries:
  - stable `id`
  - log `level`
  - rendered `message`
  - server `timestamp`
  - client-side `received_at`
- Start a log stream subscription from the authenticated app shell when the UI is shown.
- Parse the streamed response incrementally and append complete log entries in order.
- Ensure partial chunks are buffered until a full log line/message is available.

### 3. Implement the `#[server]` streaming function

- Add a colocated server function for log streaming using `#[server]`.
- Use HTTP streaming, not websocket transport.
- Prefer `#[server(protocol = Http<Json, StreamingText>)]` and return `TextStream<ServerFnError>`.
- Stream newline-delimited JSON log records so the client can decode entries incrementally.
- On subscription, send a small retained backlog first, then continue with live events.

### 4. Hook backend tracing into the stream

- Add a server-only log broadcast module under `terminal/src/backend/`.
- Capture `tracing` events from `info`, `warn`, and `error` levels.
- Ignore lower-severity events such as `debug` and `trace`.
- Install a tracing layer during server startup that forwards captured events into:
  - a small in-memory backlog
  - active live stream subscribers
- Preserve existing server logging behavior while adding this extra sink for the UI.

### 5. Define retention and disappearance behavior in the UI

- Keep log entries in display order from oldest to newest.
- If more than 10 logs are displayed, entries should begin disappearing after 3 seconds.
- If more than 100 logs are displayed, remove the 101st and older entries immediately.
- Add timed cleanup so entries expire after 3 seconds even if no new logs arrive.
- Do not expire entries by age while 10 or fewer logs are visible.

### 6. Backlog behavior

- Maintain a small in-memory backlog on the server for new subscribers.
- Default backlog size: 20 most recent log events.
- Replay the backlog immediately when a client subscribes, before live streaming begins.

### 7. Styling and rendering details

- Keep the log list visually subordinate to the active app, since the app content remains primary.
- Render each `<li>` with the log level and message.
- Add nearby styling only as needed to keep the list readable and scrollable.

### 8. Verification

- Confirm `info!`, `warn!`, and `error!` events appear in the UI stream.
- Confirm `debug!` events do not appear.
- Confirm a new subscriber receives the retained backlog and then live updates.
- Confirm log items older than 3 seconds disappear once the visible count exceeds 10.
- Confirm the list never keeps more than 100 items.
- Run `./all.sh` from the repo root after touching Rust source files during implementation.

## Interfaces To Add

- `#[server(protocol = Http<Json, StreamingText>)]`
- `async fn stream_logs() -> Result<TextStream<ServerFnError>, ServerFnError>`
- Shared serialized log payload for the UI, for example:
  - `id`
  - `level`
  - `message`
  - `timestamp`

## Assumptions

- The log panel is global to this Terminal server process for authenticated viewers.
- A small backlog means 20 retained entries.
- NDJSON over `StreamingText` is the preferred response format because it fits streaming HTTP cleanly without websockets.
