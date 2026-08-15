# Transport Guide

Deep-dive reference for MCP client transport selection and configuration. Both transports implement the `Transport` trait (`connect`, `disconnect`, `send_request`, `send_notification`).

## HttpTransport (negotiates 2026-07-28 or 2025-11-25)

**Constructor:** `HttpTransport::new(endpoint: &str) -> McpClientResult<Self>`

The default and recommended transport. Uses standard HTTP POST for all requests and handles streamed `application/json` frames in request responses. Carries whichever wire spec `McpClient::negotiate_protocol()` settles on for this connection — the transport itself doesn't hardcode a spec version.

**Key behaviors:**
- Validates `http://` or `https://` scheme on construction
- `set_protocol_version()` — called by the client after negotiation to set the `MCP-Protocol-Version` header for all subsequent requests
- `Mcp-Session-Id` header management (2025-11-25 connections only) — captured from server response, included in all subsequent requests; absent entirely on a negotiated 2026-07-28 connection, since the stateless core has no session
- `Mcp-Method` / `Mcp-Name` headers (2026-07-28) — mirrored automatically from the request body's `method` and `params.name`/`params.uri` by `apply_request_metadata_headers`; not something you set by hand
- Responses stream as concatenated `application/json` frames (progress notifications then final result); an optional SSE listener exists but `server_events` defaults to `false`
- `send_request_streaming()` — opens a long-lived POST for 2026-07-28's `subscriptions/listen`, yielding a channel of JSON payloads off the response's SSE stream; dropping the receiver closes the stream (closing the response IS cancellation on Streamable HTTP)
- JSON responses: standard `application/json` parsing
- Session cleanup via HTTP DELETE on disconnect — only sent when a session ID exists (2025-11-25 fallback); a no-op on 2026-07-28

**Capabilities:**
```rust
TransportCapabilities {
    streaming: true,
    bidirectional: false,
    server_events: false,
    max_message_size: None,
    persistent: false,
}
```

**When to use:** Any current server. This is the default when `with_url()` auto-detects, and the only transport that reaches 2026-07-28.

**Alternative constructor:** `HttpTransport::with_client(endpoint, reqwest_client)` — bring your own `reqwest::Client` for custom TLS, proxies, or connection pooling.

## SseTransport (Legacy HTTP+SSE — MCP 2024-11-05)

**Constructor:** `SseTransport::new(endpoint: &str) -> McpClientResult<Self>`

Legacy transport for servers implementing MCP 2024-11-05 or earlier. Uses a two-endpoint model: one for SSE event streaming, one for HTTP POST requests.

**Key behaviors:**
- Validates `http://` or `https://` scheme
- `connect()` is a no-op (marks transport ready); SSE listener starts lazily via `start_event_listener()`
- Aborts the SSE listener task on `disconnect()` or `Drop`
- Separate SSE endpoint URL derived from the base endpoint

**Capabilities:**
```rust
TransportCapabilities {
    streaming: true,
    bidirectional: false,
    server_events: true,
    max_message_size: None,
    persistent: true,  // SSE connection is long-lived
}
```

**When to use:** Only for legacy MCP servers that require the two-endpoint SSE model.

**Alternative constructor:** `SseTransport::with_endpoints(endpoint, sse_endpoint)` — specify separate POST and SSE endpoints when they differ.

## Auto-Detection Logic

`TransportFactory::from_url(url)` (used internally by `McpClientBuilder::with_url()`) applies these rules:

```
fn detect_transport_type(url: &str) -> TransportType:
    if url path contains "/sse" OR query contains "transport=sse":
        → TransportType::Sse
    else:
        → TransportType::Http
```

**Examples:**
| URL | Detected Transport |
|---|---|
| `http://localhost:8080/mcp` | `HttpTransport` |
| `http://localhost:8080/api/mcp` | `HttpTransport` |
| `http://localhost:8080/sse` | `SseTransport` |
| `http://localhost:8080/mcp?transport=sse` | `SseTransport` |

## Transport Trait

Both transports implement:

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    fn transport_type(&self) -> TransportType;
    fn capabilities(&self) -> TransportCapabilities;
    async fn connect(&self) -> McpClientResult<()>;
    async fn disconnect(&self) -> McpClientResult<()>;
    fn is_connected(&self) -> bool;
    async fn send_request(&self, request: Value) -> McpClientResult<Value>;
    async fn send_request_streaming(&self, request: Value)
        -> McpClientResult<tokio::sync::mpsc::UnboundedReceiver<Value>>; // default: unsupported
    async fn send_request_with_headers(&self, request: Value) -> McpClientResult<TransportResponse>;
    async fn send_notification(&self, notification: Value) -> McpClientResult<()>;
    async fn send_delete(&self, session_id: &str) -> McpClientResult<()>;
    fn set_session_id(&self, session_id: String);
    fn clear_session_id(&self);
    async fn start_event_listener(&self) -> McpClientResult<EventReceiver>;
    fn connection_info(&self) -> ConnectionInfo;
}
```

Custom transports can implement this trait and pass to `McpClientBuilder::with_transport()`. `send_request_streaming` only needs a real implementation to support 2026-07-28's `subscriptions/listen`; the default returns "unsupported" — fine for a transport that only ever negotiates 2025-11-25 or 2024-11-05.

## Feature Comparison

| Feature | HttpTransport | SseTransport |
|---|---|---|
| MCP protocol version | 2026-07-28 or 2025-11-25, negotiated per connection | 2024-11-05 only |
| Request method | POST | POST |
| Server events | In-response SSE | Separate SSE endpoint |
| Session ID | `Mcp-Session-Id` header (2025-11-25 connections only; absent on 2026-07-28) | `Mcp-Session-Id` header |
| `subscriptions/listen` (2026-07-28) | Supported via `send_request_streaming` | Not supported |
| Connection model | Request/response | Long-lived SSE + POST (listener starts lazily) |
| Background tasks | None | SSE listener task (lazy, via `start_event_listener()`) |
| Disconnect cleanup | HTTP DELETE (2025-11-25 fallback only; no-op on 2026-07-28) | Abort SSE task |
| Custom `reqwest::Client` | `with_client()` | Not supported |
| Custom endpoints | N/A | `with_endpoints()` |
