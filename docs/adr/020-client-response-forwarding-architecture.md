# ADR-020: Client Response Forwarding Architecture

**Status**: Accepted

**Date**: 2026-03-05

## Context

MCP 2025-11-25 introduces server-initiated requests: the server sends a JSON-RPC
request (with `method` + `id`) to the client, expecting a JSON-RPC response back.
Two protocol methods use this pattern:

- `sampling/createMessage` — server asks client to generate a message via its LLM
- `elicitation/create` — server asks client to collect user input

The client's `StreamHandler` already received these requests via the SSE event stream
and dispatched them to a user-registered callback (`on_request`). However, the
callback's return value (`Result<Value, String>`) was logged and discarded — no
response was ever sent back to the server. This violated the bidirectional
request/response contract and caused servers to hang indefinitely.

### Constraints

1. **Transport ownership**: `BoxedTransport` lives behind
   `Arc<tokio::sync::Mutex<BoxedTransport>>` in `McpClient`. The `StreamHandler`
   runs in a `tokio::spawn`-ed task. Sharing the transport directly into the
   handler task creates lock contention with the main request flow.

2. **Drop safety**: `McpClient` implements `Drop` for cleanup. `Drop` cannot
   `.await`, so the response consumer task handle must be manageable synchronously.

3. **Event classification**: The HTTP transport SSE parser must distinguish three
   frame types from a single JSON stream:
   - **Request** (server→client): has both `method` and `id`
   - **Notification** (server→client): has `method` but no `id`
   - **Response** (client←server): has `id` but no `method` (async response to
     a client-originated request)

   The prior implementation checked `method` before `id`, misclassifying
   server-initiated requests (which have both) as notifications.

## Decision

### Channel-based response forwarding

Add an `mpsc::UnboundedSender<Value>` to `StreamHandler`. When the callback returns,
construct a JSON-RPC 2.0 response and send it through the channel. A separate
consumer task in `McpClient` drains the channel and calls
`transport.send_notification()` for each response.

```
Transport → event_sender → StreamHandler → callback → response_sender → consumer task → transport.send_notification()
```

**Why a channel, not `Arc<Mutex<Transport>>`**: The transport is already behind
`Arc<tokio::sync::Mutex<BoxedTransport>>`. Sharing it into the StreamHandler's
spawned task would create lock contention with the main client request flow (every
`list_tools`, `call_tool`, etc. also locks the transport). The channel decouples the
two — the stream handler fires responses without blocking, and the consumer task
acquires the transport lock independently.

**Why `send_notification()`**: JSON-RPC responses are fire-and-forget from the
client's perspective — there is no reply to a reply. `send_notification()` sends
arbitrary JSON without expecting a response, which is exactly what we need.

### Consumer task lifecycle

The consumer task's `JoinHandle` is stored in
`Arc<parking_lot::Mutex<Option<JoinHandle<()>>>>`. `parking_lot::Mutex` is used
instead of `std::sync::Mutex` because:

- `Drop` cannot `.await`, so async mutex is unusable in the cleanup path
- `parking_lot::Mutex` does not poison on panic (no `.unwrap()` on `.lock()`)
- The lock is held only briefly (take/replace a handle), so blocking is negligible

Lifecycle:
- `connect()`: abort any existing consumer, spawn new one, store handle
- `disconnect()`: take handle, abort
- `Drop`: take handle, abort (synchronous, no `.await`)

### ServerEvent::Response variant

Added `ServerEvent::Response(Value)` to distinguish id-only frames (responses to
client-originated requests) from server-initiated requests (which have both `method`
and `id`). The StreamHandler ignores `Response` events — they are handled by the
normal request/response matching path.

### Event classification fix (HTTP transport)

Changed the SSE event classification order:

1. `method` + `id` present → `ServerEvent::Request`
2. `method` only → `ServerEvent::Notification`
3. `id` only → `ServerEvent::Response`

The SSE transport (`transport/sse.rs`) was not affected — it uses the SSE `event:`
field for classification.

### JSON-RPC response construction

- Success: `{"jsonrpc":"2.0", "id":<original_id>, "result":<callback_return>}`
- Error: `{"jsonrpc":"2.0", "id":<original_id>, "error":{"code":-32603, "message":<error_string>}}`
- No callback registered: `{"jsonrpc":"2.0", "id":<original_id>, "error":{"code":-32601, "message":"Method not found: no request handler configured"}}`
- Missing/null `id`: callback invoked but no response emitted (per JSON-RPC 2.0 spec)

## Consequences

### Positive

- **Spec compliance**: Server-initiated sampling and elicitation requests now
  receive responses. Servers using `createMessage` or `elicit` against this client
  will no longer hang.
- **No lock contention**: The channel decouples StreamHandler from transport access.
  The consumer task acquires the transport lock independently of the main request
  flow.
- **Clean shutdown**: `parking_lot::Mutex` + `JoinHandle::abort()` provides
  synchronous cleanup in `Drop` without panic risk from mutex poisoning.
- **Backward compatible**: No public API changes. Existing code that doesn't use
  `on_request` callbacks is unaffected. The `ServerEvent::Response` variant is
  additive.

### Negative

- **Indirect transport path**: Responses flow through a channel and consumer task
  rather than being sent directly from the callback. This adds a small amount of
  latency (channel send + task wake + transport lock acquisition). Acceptable for
  the expected throughput of sampling/elicitation requests.
- **`send_notification()` semantics**: The consumer calls `send_notification()` to
  send JSON-RPC responses, which is semantically odd (it's a "response", not a
  "notification"). However, at the transport level both are identical: send JSON
  without expecting a reply.

### Risks

- **Wire-format coverage gap**: Current tests validate the pipeline with in-process
  channels and a mock transport, not over live HTTP/SSE. A regression in HTTP
  framing or content-type negotiation specific to response forwarding would not be
  caught. Tracked as P2 in TODO_TRACKER.md for future live integration test.

## Implementation

### Key File Paths

| Component | Path |
|-----------|------|
| StreamHandler (response channel) | `crates/turul-mcp-client/src/streaming.rs` |
| McpClient (consumer task) | `crates/turul-mcp-client/src/client.rs` |
| ServerEvent::Response variant | `crates/turul-mcp-client/src/transport.rs` |
| HTTP event classification fix | `crates/turul-mcp-client/src/transport/http.rs` |
| Pipeline integration tests | `tests/client_server_request_response.rs` |
| Mock transport tests | `crates/turul-mcp-client/src/client.rs` (test module) |

### Test Coverage

| Test | What it validates |
|------|-------------------|
| `test_stream_handler_sends_success_response` | StreamHandler produces correct success JSON-RPC on channel |
| `test_stream_handler_sends_error_response` | StreamHandler produces correct error JSON-RPC on channel |
| `test_stream_handler_no_callback_sends_method_not_found` | Missing callback → -32601 error response |
| `test_stream_handler_no_id_skips_response` | Request without `id` → callback invoked, no response |
| `test_stream_handler_null_id_skips_response` | Request with `null` id → no response |
| `test_server_request_response_pipeline_success` | Full StreamHandler pipeline: request → callback → response (success) |
| `test_server_request_response_pipeline_error` | Full StreamHandler pipeline: request → callback → response (error) |
| `test_server_request_response_pipeline_multiple_requests` | Sequential requests with id correlation |
| `test_response_event_does_not_trigger_callback` | `ServerEvent::Response` ignored by callback path |
| `test_client_response_consumer_pipeline` | McpClient consumer task: mock transport verifies response reaches `send_notification()` |
| `test_client_response_consumer_pipeline_error` | McpClient consumer task: error response reaches `send_notification()` |

## See Also

- [ADR-005: MCP Message Notifications Architecture](./005-mcp-message-notifications-architecture.md) — SSE notification delivery
- [ADR-006: Streamable HTTP Compatibility](./006-streamable-http-compatibility.md) — SSE streaming and chunked transfer
