# Transport Guide

Deep-dive reference for MCP client transport selection and configuration. Both transports implement the `Transport` trait (`connect`, `disconnect`, `send_request`, `send_notification`).

## HttpTransport (Streamable HTTP — MCP 2025-11-25)

**Constructor:** `HttpTransport::new(endpoint: &str) -> McpClientResult<Self>`

The default and recommended transport. Uses standard HTTP POST for all requests and handles SSE streaming for server-initiated events within the response body.

**Key behaviors:**
- Validates `http://` or `https://` scheme on construction
- Automatic `Mcp-Session-Id` header management — captured from server response, included in all subsequent requests
- Handles chunked SSE responses: parses `text/event-stream` content type, queues `ServerEvent` notifications for the stream handler
- JSON responses: standard `application/json` parsing
- Session cleanup via HTTP DELETE on disconnect

**Capabilities:**
```rust
TransportCapabilities {
    streaming: true,
    bidirectional: false,
    server_events: true,
    max_message_size: None,
    persistent: false,
}
```

**When to use:** Any MCP 2025-11-25 server. This is the default when `with_url()` auto-detects.

**Alternative constructor:** `HttpTransport::with_client(endpoint, reqwest_client)` — bring your own `reqwest::Client` for custom TLS, proxies, or connection pooling.

## SseTransport (Legacy HTTP+SSE — MCP 2024-11-05)

**Constructor:** `SseTransport::new(endpoint: &str) -> McpClientResult<Self>`

Legacy transport for servers implementing MCP 2024-11-05 or earlier. Uses a two-endpoint model: one for SSE event streaming, one for HTTP POST requests.

**Key behaviors:**
- Validates `http://` or `https://` scheme
- Spawns a background Tokio task for SSE event listening on `connect()`
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
    async fn connect(&mut self) -> McpClientResult<()>;
    async fn disconnect(&mut self) -> McpClientResult<()>;
    fn is_connected(&self) -> bool;
    async fn send_request(&mut self, request: Value) -> McpClientResult<Value>;
    async fn send_request_with_headers(&mut self, request: Value) -> McpClientResult<TransportResponse>;
    async fn send_notification(&mut self, notification: Value) -> McpClientResult<()>;
    async fn send_delete(&mut self, session_id: &str) -> McpClientResult<()>;
    fn set_session_id(&mut self, session_id: String);
    async fn start_event_listener(&mut self) -> McpClientResult<EventReceiver>;
    fn connection_info(&self) -> ConnectionInfo;
}
```

Custom transports can implement this trait and pass to `McpClientBuilder::with_transport()`.

## Feature Comparison

| Feature | HttpTransport | SseTransport |
|---|---|---|
| MCP protocol version | 2025-11-25 | 2024-11-05 |
| Request method | POST | POST |
| Server events | In-response SSE | Separate SSE endpoint |
| Session ID | `Mcp-Session-Id` header | Connection-based |
| Connection model | Request/response | Long-lived SSE + POST |
| Background tasks | None | SSE listener task |
| Disconnect cleanup | HTTP DELETE | Abort SSE task |
| Custom `reqwest::Client` | `with_client()` | Not supported |
| Custom endpoints | N/A | `with_endpoints()` |
