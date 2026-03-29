# Streaming Logs Through The Current Remote

## Goal

Make the logs panel follow the currently selected remote instead of always streaming from the local server.

Today the client opens `crate::logs::stream::stream()` with no remote argument, so the stream always terminates in the local server implementation in [`terminal/src/logs/stream.rs`](/home/richard/Github/Terminal/terminal/src/logs/stream.rs). The UI entry point in [`terminal/src/logs/client/panel.rs`](/home/richard/Github/Terminal/terminal/src/logs/client/panel.rs) also has no `XSignal<Remote>` input, so it cannot react to remote selection changes.

## How `notify_service` Works

`notify_service` is the closest existing pattern for streaming server data through the gateway to another client:

- The server function entry point is [`terminal/src/text_editor/notify/server_fn.rs`](/home/richard/Github/Terminal/terminal/src/text_editor/notify/server_fn.rs). It receives a stream from the browser and hands it to `notify_dispatch`.
- `notify_dispatch` in [`terminal/src/backend/client_service/notify_service/dispatch.rs`](/home/richard/Github/Terminal/terminal/src/backend/client_service/notify_service/dispatch.rs) peeks the first message, extracts the remote address, and chooses local vs remote execution.
- The routing itself uses the generic `DistributedCallback` helper in [`terminal/src/backend/client_service/routing.rs`](/home/richard/Github/Terminal/terminal/src/backend/client_service/routing.rs).
- The remote branch in [`terminal/src/backend/client_service/notify_service/callback.rs`](/home/richard/Github/Terminal/terminal/src/backend/client_service/notify_service/callback.rs) prepends the remaining hop address, opens a tonic client, and forwards the stream to the remote client.
- The tonic server implementation is in [`terminal/src/backend/client_service/notify_service/grpc.rs`](/home/richard/Github/Terminal/terminal/src/backend/client_service/notify_service/grpc.rs), and the client-side tunnel service is registered in [`terminal/src/backend/agent.rs`](/home/richard/Github/Terminal/terminal/src/backend/agent.rs).

For logs we only need the response stream, not a bi-directional request stream. That means we can reuse the same routing idea, but the actual service can be much simpler:

- one request containing the remote address / routing target
- one streamed response carrying log lines
- no client-originated stream after startup

## Proposed Shape

### 1. Make the logs UI remote-aware

- Change `crate::logs::panel()` to `crate::logs::panel(remote: XSignal<Remote>)`.
- Update the call site in [`terminal/src/frontend/login.rs`](/home/richard/Github/Terminal/terminal/src/frontend/login.rs) to pass the existing `remote` signal into the logs panel.
- Thread the selected remote into `LogsEngine::new(...)` in [`terminal/src/logs/client/engine.rs`](/home/richard/Github/Terminal/terminal/src/logs/client/engine.rs).
- No explicit restart logic should be needed inside `LogsEngine`: when the selected remote changes, the `#[signal] remote` render path already rebuilds the relevant UI subtree, which drops the old `LogsEngine` and creates a new one for the new remote.

### 2. Change the server function to accept a remote

- Update [`terminal/src/logs/stream.rs`](/home/richard/Github/Terminal/terminal/src/logs/stream.rs) so `stream` takes `remote: Option<ClientAddress>`.
- Keep the current local implementation as the local execution path, but move it behind a dispatch layer instead of calling `stream_impl()` directly.
- The local branch should still produce the same NDJSON lines backed by `LogState::get().subscribe()`.

This is the key signature change that unblocks the rest:

```rust
pub async fn stream(
    remote: Option<ClientAddress>,
) -> Result<TextStream<ServerFnError>, ServerFnError>
```

### 3. Add `terminal/src/backend/client_service/logs_service`

Create a new folder:

- [`terminal/src/backend/client_service/logs_service`](/home/richard/Github/Terminal/terminal/src/backend/client_service/logs_service)

This should mirror the layering of `notify_service`, but trimmed for one-way streaming:

- `mod.rs`
  Expose the service module under a `logs-panel` feature gate.
- `dispatch.rs`
  Accept the selected remote and return a hybrid/local response stream.
- `callback.rs`
  Implement `DistributedCallback` for logs.
- `grpc.rs`
  Expose the tonic `LogsService` implementation on `ClientServiceImpl`.
- `response.rs` and `response/remote.rs`
  Hold the local/remote response stream wrapper types if needed.

I would avoid request-stream plumbing unless it proves necessary. Unlike notify, logs can route from a plain unary request into a streaming response.

### 4. Introduce a gRPC logs proto

Add a new proto, likely:

- [`terminal/src/backend/protos/logs.proto`](/home/richard/Github/Terminal/terminal/src/backend/protos/logs.proto)

Suggested shape:

```proto
service LogsService {
  rpc StreamLogs(LogsRequest) returns (stream LogsResponse);
}

message LogsRequest {
  terrazzo.shared.ClientAddress address = 1;
}

message LogsResponse {
  string line = 1;
}
```

Then wire it through:

- [`terminal/build.rs`](/home/richard/Github/Terminal/terminal/build.rs)
- [`terminal/src/backend/protos/mod.rs`](/home/richard/Github/Terminal/terminal/src/backend/protos/mod.rs)
- [`terminal/src/backend/agent.rs`](/home/richard/Github/Terminal/terminal/src/backend/agent.rs)

`notify_service` uses a client-streaming request because it needs `Start`, `Watch`, and `UnWatch`. Logs do not, so a unary request plus streaming response should be enough.

### 5. Route the server function through the new service

The updated `stream(remote)` server function should:

- call `logs_service::dispatch::logs_dispatch(remote.unwrap_or_default())`
- choose local execution when the address is empty
- otherwise open a gRPC `LogsServiceClient` to the next hop and stream lines back
- convert the resulting local/remote stream into `TextStream<ServerFnError>`

The local callback can keep using the current serializer:

- subscribe to `LogState`
- serialize each `LogEvent` as one JSON line
- return `TextStream::new(stream.map(Ok))`

The remote callback should only forward already-serialized lines. That keeps the on-wire contract simple and preserves the existing browser parser in [`terminal/src/logs/client/engine.rs`](/home/richard/Github/Terminal/terminal/src/logs/client/engine.rs).

## Implementation Order

1. Pass `remote: XSignal<Remote>` into the logs panel and engine.
2. Change `logs::stream::stream` to accept `Option<ClientAddress>`.
3. Add `logs.proto` and register the generated tonic service.
4. Create `backend/client_service/logs_service` with dispatch, callback, grpc, and response wrappers.
5. Swap the server function from direct local subscription to remote-aware dispatch.
6. Verify that switching remotes while the panel is open tears down the old stream and attaches to the new one.

## Important Details

- Feature gates should follow the existing logs feature, likely `logs-panel`, the same way `notify_service` is gated by `text-editor`.
- `ClientServiceImpl` registration in [`terminal/src/backend/agent.rs`](/home/richard/Github/Terminal/terminal/src/backend/agent.rs) must include the new tonic service only when the logs feature is enabled.
- The browser-side `NdjsonBuffer` parser should not need to change if the gRPC service forwards complete JSON lines verbatim.
- Error mapping should follow the `notify_service` pattern: local errors become `ServerFnError`, remote tonic failures become `Status`, and the dispatch layer converts them into the server-function error type returned to the browser.

## Minimal Viable Architecture

The smallest safe version is:

- UI passes `remote`
- server function takes `remote`
- new unary-request/server-streaming gRPC `LogsService`
- local callback reuses current `LogState` subscription logic
- remote callback forwards `String` lines

That should be enough to make logs follow the selected remote without copying the full complexity of `notify_service`.
