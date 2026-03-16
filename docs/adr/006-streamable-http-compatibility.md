# ADR-006: MCP Streamable HTTP Transport Compatibility Architecture

**Status**: Accepted (amended 2026-03-16)
**Date**: 2025-09-02
**Authors**: Claude Code via turul-mcp-framework development

## Context

The turul-mcp-framework implements MCP (Model Context Protocol) 2025-06-18 Streamable HTTP Transport, which replaces the deprecated HTTP+SSE transport. However, real-world MCP clients vary in their specification compliance, particularly around HTTP Accept header handling.

### Key Challenges

1. **MCP Inspector Non-Compliance**: MCP Inspector (official tooling) sends only `Accept: application/json` instead of the spec-required `Accept: application/json, text/event-stream` for POST requests
2. **Timeout Issues**: Non-compliant clients experience -32001 timeout errors when servers attempt SSE responses
3. **Notification Delivery**: Server-initiated notifications fail when sent to non-existent GET SSE connections
4. **Configuration Control**: Users need ability to disable POST SSE entirely for compatibility testing

### MCP 2025-06-18 Specification Requirements

Per the official specification:

> **POST Requests**: Clients MUST include both `application/json` and `text/event-stream` in Accept header to receive streaming responses:
> ```
> Accept: application/json, text/event-stream
> ```

> **Dual-Path Delivery**: 
> - POST requests: Request/response with optional streaming
> - GET SSE: Persistent server-initiated message stream
> - Both paths use same session ID via `Mcp-Session-Id` header

## Decision

Implement a **compatibility-first architecture** that gracefully handles both compliant and non-compliant MCP clients through intelligent Accept header parsing and server configuration controls.

## Solution Architecture

### 1. Accept Header Compliance Detection

```rust
#[derive(Debug, Clone, PartialEq)]
enum AcceptMode {
    Compliant,  // Both application/json and text/event-stream
    JsonOnly,   // Only application/json (MCP Inspector case)
    SseOnly,    // Only text/event-stream  
    Invalid,    // Neither or malformed
}

fn parse_mcp_accept_header(accept_header: &str) -> (AcceptMode, bool) {
    let accepts_json = accept_header.contains("application/json") || accept_header.contains("*/*");
    let accepts_sse = accept_header.contains("text/event-stream");
    
    let mode = match (accepts_json, accepts_sse) {
        (true, true) => AcceptMode::Compliant,
        (true, false) => AcceptMode::JsonOnly,
        (false, true) => AcceptMode::SseOnly,
        (false, false) => AcceptMode::Invalid,
    };
    
    let should_use_sse = match mode {
        AcceptMode::Compliant => true,     // Server chooses based on method + config
        AcceptMode::JsonOnly => false,     // Force JSON for compatibility
        AcceptMode::SseOnly => true,       // Force SSE if server allows
        AcceptMode::Invalid => false,      // Fallback to JSON
    };
    
    (mode, should_use_sse)
}
```

### 2. SSE Decision Logic

```rust
let should_use_sse = match accept_mode {
    AcceptMode::JsonOnly => false,    // Force JSON for compatibility (MCP Inspector)
    AcceptMode::Invalid => false,     // Fallback to JSON for invalid headers
    AcceptMode::Compliant => self.config.enable_sse && accepts_sse && is_tool_call, 
    AcceptMode::SseOnly => self.config.enable_sse && accepts_sse,
};
```

### 3. Notification Delivery with Connection Awareness

```rust
pub async fn broadcast_to_session_with_options(
    &self,
    session_id: &str,
    event_type: String,
    data: Value,
    store_when_no_connections: bool,
) -> Result<u64, StreamError> {
    if !store_when_no_connections && !self.has_connections(session_id).await {
        debug!("🚫 Suppressing notification for session {} (no connections)", session_id);
        return Err(StreamError::NoConnections(session_id.to_string()));
    }
    // ... continue with broadcast
}
```

### 4. Server Configuration Control

```rust
// Server builder supports SSE enable/disable
let server = McpServer::builder()
    .name("test-server")
    .sse(post_sse_enabled)  // Respects --disable-post-sse flag
    .build()?;
```

## Implementation Details

### Accept Header Compatibility Matrix

| Client Accept Header | Mode | SSE Used? | Rationale |
|---------------------|------|-----------|-----------|
| `application/json, text/event-stream` | Compliant | Method-dependent* | See Content-Type Negotiation Policy below |
| `application/json` | JsonOnly | No | MCP Inspector compatibility |
| `text/event-stream` | SseOnly | Yes | Client demands SSE (Claude Desktop pattern) |
| `*/*` | JsonOnly | No | Treated as JSON-only for safety |
| Invalid/Empty | Invalid | No | Fallback to JSON |

*See next section for method-level policy.

### Content-Type Negotiation Policy (Added 2026-03-16)

When the client sends `Accept: application/json, text/event-stream` (combined), the server uses `should_use_sse(method)` to choose the response format:

| JSON-RPC Method | Response Content-Type | Rationale |
|---|---|---|
| `tools/call` | `text/event-stream` | May emit `notifications/progress` mid-stream |
| `sampling/createMessage` | `text/event-stream` | May emit mid-stream events |
| `elicitation/create` | `text/event-stream` | May emit mid-stream events |
| All other methods | `application/json` | Guaranteed single-response, no mid-stream events |

**This is a conservative transport heuristic, not a proven optimal policy.** Key limitations:

1. **Method-level, not tool-level.** Every `tools/call` under combined Accept gets SSE — even simple tools that never call `notify_progress()`. The transport layer does not currently have per-tool progress metadata.
2. **Architectural constraint, not fundamental.** Per-tool metadata (e.g., "this tool never emits progress") could be plumbed from the tool registry to the transport layer, enabling finer decisions. This is not implemented.
3. **SSE framing cost.** Non-streaming `tools/call` responses pay unnecessary SSE framing overhead (`data: {json}\n\n` vs raw JSON) and may hit client/proxy SSE quirks.
4. **Progress notifications are independent of tasks.** `notify_progress()` flows through the `StreamManager` as SSE events regardless of whether the server has a task runtime configured. Tasks are a separate system for background execution — they are not the right discriminator for this policy.

**Implementation**: `StreamableHttpContext::should_use_sse()` in `crates/turul-http-mcp-server/src/streamable_http.rs`.

**Test coverage**: `tests/content_type_negotiation.rs` — 4 tests asserting Content-Type matches body format for JSON-only, SSE-only, combined+tools/call, and combined+tools/list Accept patterns.

### Configuration Precedence

1. **Server Configuration**: `enable_sse=false` disables all SSE usage
2. **Accept Header Compliance**: Non-compliant headers force JSON responses
3. **Method Type**: `should_use_sse()` heuristic for combined Accept (see above)
4. **Connection State**: Notifications require active GET SSE connections

## Benefits

### 1. MCP Inspector Compatibility
- ✅ Handles `Accept: application/json` gracefully
- ✅ No timeout errors from unexpected SSE responses  
- ✅ Standard JSON responses for all operations

### 2. Specification Compliance
- ✅ Full MCP 2025-06-18 Streamable HTTP support
- ✅ Proper dual-path delivery (POST + GET SSE)
- ✅ Session-aware notification routing

### 3. Developer Experience
- ✅ `--disable-post-sse` flag works correctly
- ✅ Clear debug logging for decision points
- ✅ Graceful fallback behaviors

### 4. Performance & Reliability
- ✅ No wasted notifications to non-existent connections
- ✅ Configurable SSE behavior per deployment needs
- ✅ Predictable response types based on headers

## Edge Cases Handled

### Non-Compliant MCP Inspector
```
Request: Accept: application/json
Response: content-type: application/json (never SSE)
Behavior: Standard JSON-RPC responses, no timeouts
```

### Compliant MCP Client with Server SSE Disabled  
```
Request: Accept: application/json, text/event-stream
Server: --disable-post-sse
Response: content-type: application/json (SSE disabled)
Behavior: Respects server configuration over client capabilities
```

### Mixed Environment Testing
```
Same server supports:
- MCP Inspector → JSON-only responses
- Compliant clients → SSE streaming (when enabled)
- Debug tooling → Configurable behavior via flags
```

## Monitoring & Debug

### Debug Logs Provide Full Visibility
```
Decision point: method=Some("tools/call"), accept_mode=Compliant, 
accepts_sse=true, server_sse_enabled=false, session_id=..., is_tool_call=true
📄 Returning standard JSON response (mode: Compliant) for method: Some("tools/call")
```

### Key Metrics Logged
- Accept header parsing results
- SSE decision logic outcomes  
- Server configuration state
- Connection existence checks
- Notification delivery success/failures

## AWS Lambda Runtime Compatibility

### Lambda Streaming Limitation (Updated 2025-10-02)

**Status**: Known limitation - documented but not fixed

**Problem**: Server-initiated notifications (via `session.notify_progress()`) do not reach clients in AWS Lambda deployments. The `StreamableHttpHandler` spawns background tasks (`tokio::spawn`) to forward notifications, but Lambda's execution model tears down the invocation immediately after the handler returns, killing spawned tasks before notifications can be sent.

**What Works in Lambda**:
- ✅ Tool calls execute correctly via `StreamableHttpHandler`
- ✅ Synchronous request/response operations complete successfully
- ✅ All MCP protocol operations except server-initiated notifications

**What Doesn't Work in Lambda**:
- ❌ Server-initiated progress notifications (`session.notify_progress()`)
- ❌ Background task-based notification delivery
- ❌ POST SSE streaming for notifications

**Why Tool Calls Work**: Tool execution completes synchronously within the Lambda handler's lifetime. The response is fully assembled and returned before Lambda tears down the invocation, so no background tasks are required.

**Why Notifications Don't Work**: Notifications require background tasks to forward messages to the SSE stream. Lambda kills these tasks when the handler returns, preventing notification delivery.

**Decision**: Accept this limitation and document it clearly. The attempted fix (environment detection + routing to SessionMcpHandler) broke working tool calls, causing -32001 timeout errors. The simple solution is to use `StreamableHttpHandler` for all protocol ≥ 2025-03-26 requests and document that notifications don't work in Lambda.

### Routing Logic (Restored 2025-10-02)

Simple protocol-version-based routing without environment detection:

```rust
// Route based on protocol version only
let hyper_resp = if protocol_version.supports_streamable_http() {
    debug!("Using StreamableHttpHandler for protocol {}", protocol_version.to_string());
    self.streamable_handler.handle_request(hyper_req).await
} else {
    debug!("Using SessionMcpHandler for legacy protocol {}", protocol_version.to_string());
    self.session_handler.handle_mcp_request(hyper_req).await?
};
```

**No Lambda Detection**: All Lambda detection code (environment variable checks, capability clamping, routing guards) has been removed. The framework uses simple protocol-version-based routing everywhere.

### Lambda Compatibility Matrix

| Environment | Protocol Version | Handler Used | Tool Calls | Server Notifications |
|-------------|------------------|--------------|------------|---------------------|
| Lambda | ≥ 2025-03-26 | StreamableHttpHandler | ✅ Works | ❌ Known limitation |
| Lambda | ≤ 2024-11-05 | SessionMcpHandler | ✅ Works | ❌ N/A (legacy) |
| Non-Lambda | ≥ 2025-03-26 | StreamableHttpHandler | ✅ Works | ✅ Works |
| Non-Lambda | ≤ 2024-11-05 | SessionMcpHandler | ✅ Works | ❌ N/A (legacy) |

### Alternatives for Notification Support

For deployments requiring server-initiated notifications:

- **AWS Fargate**: Container-based, supports long-running processes with background tasks
- **ECS**: Full container orchestration with persistent connections
- **EC2**: Traditional server deployment with complete streaming support
- **Cloud Run (GCP)**: Container platform with streaming response support
- **Standard HTTP Server**: Any long-running server process (not serverless)

## Future Considerations

### Protocol Evolution
- Architecture supports future MCP specification changes
- Accept header logic can be extended for new content types
- Configuration system scales to additional transport options

### Per-Tool Content-Type Negotiation
- The current method-based `should_use_sse()` heuristic could be refined with per-tool progress metadata
- The tool registry knows whether a tool declares `task_support` or calls `notify_progress()` — plumbing this to the transport layer would allow `tools/call` responses for non-streaming tools to use `application/json` under combined Accept
- This is an optimization, not a correctness fix — the current conservative policy is spec-compliant

### Client Ecosystem Maturity
- As clients become more compliant, JsonOnly fallback usage should decrease
- Server operators can monitor compliance via debug logs
- Migration path exists to stricter compliance enforcement

### Lambda Streaming Future
- AWS Lambda may add support for persistent connections/streaming responses
- If AWS adds this capability, server-initiated notifications may work without code changes
- Monitor AWS Lambda roadmap for streaming response support

## MCP Streaming Terminology

### Critical Distinction: Two Different Mechanisms

**Important**: "SSE" and "Streamable HTTP" are NOT interchangeable terms. They refer to different transport mechanisms with different purposes.

### POST Streamable HTTP (Tool Progress Notifications)

**What it is**: MCP 2025-06-18 transport mechanism where POST requests receive chunked HTTP responses containing progress notifications + final result.

**Request Pattern**:
```http
POST /mcp HTTP/1.1
Accept: text/event-stream
Content-Type: application/json
Mcp-Session-Id: {session-id}

{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{...}}
```

**Response Pattern**:
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Transfer-Encoding: chunked

data: {"jsonrpc":"2.0","method":"notifications/progress","params":{...}}

data: {"jsonrpc":"2.0","id":1,"result":{...}}
```

**Key Characteristics**:
- ✅ Request-response model (client sends POST, gets chunked response)
- ✅ Single connection per request (terminates after final result)
- ✅ Tool progress notifications + final result in same stream
- ✅ Works on serverless (Lambda, Cloud Run) with buffering
- ✅ Uses `text/event-stream` framing for chunks
- ❌ NOT a long-lived connection
- ❌ NOT bidirectional

**Code Path**:
- Handler: `StreamableHttpHandler::handle_post_streamable_http()`
- Entry: POST to `/mcp` with `Accept: text/event-stream`
- Used for: Tool execution with progress updates

### GET SSE (Server-Initiated Notifications)

**What it is**: Traditional Server-Sent Events - a long-lived GET request that streams server-initiated notifications to the client.

**Request Pattern**:
```http
GET /mcp HTTP/1.1
Accept: text/event-stream
Mcp-Session-Id: {session-id}
```

**Response Pattern**:
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Transfer-Encoding: chunked

data: {"jsonrpc":"2.0","method":"notifications/message","params":{...}}

data: {"jsonrpc":"2.0","method":"notifications/cancelled","params":{...}}

[stream continues indefinitely]
```

**Key Characteristics**:
- ✅ Long-lived connection (client keeps GET open)
- ✅ Server→client only (unidirectional)
- ✅ For background server notifications (not tied to specific request)
- ✅ Uses standard SSE protocol
- ❌ NOT request-response pattern
- ❌ NOT for tool progress (use POST Streamable HTTP instead)
- ❌ NOT serverless-friendly (requires long-lived process)

**Code Path**:
- Handler: `StreamableHttpHandler::handle_get_sse_notifications()`
- Entry: GET to `/mcp` with `Accept: text/event-stream`
- Used for: Background server notifications, subscription updates

### Terminology Comparison Table

| Feature | POST Streamable HTTP | GET SSE |
|---------|---------------------|---------|
| **HTTP Method** | POST | GET |
| **Accept Header** | `text/event-stream` | `text/event-stream` |
| **Connection Lifetime** | Request-response (short) | Long-lived |
| **Direction** | Client→Server (request)<br>Server→Client (chunked response) | Server→Client only |
| **Purpose** | Tool progress + result delivery | Background server notifications |
| **MCP Methods** | `tools/call`, `resources/read`, etc. | Server-initiated messages |
| **Serverless Compatible** | ✅ Yes (with buffering) | ❌ No (requires long-lived process) |
| **Code Handler** | `handle_post_streamable_http()` | `handle_get_sse_notifications()` |
| **Example Use Case** | Progress bar during long tool execution | Server notifying client of external event |

### Documentation Standards

When writing about streaming in this framework, always specify:

1. **Method**: POST or GET?
2. **Purpose**: Tool progress or server notifications?
3. **Lifetime**: Request-response or long-lived?

**✅ GOOD Examples**:
- "POST Streamable HTTP tool progress notifications are broken in Lambda due to background task teardown"
- "GET SSE stream for server notifications works correctly in long-running HTTP servers"
- "The `handle_post_streamable_http()` method implements chunked responses for tool execution"

**❌ BAD Examples**:
- "SSE is broken in Lambda" (which SSE?)
- "Streaming doesn't work" (which streaming mechanism?)
- "The streaming handler" (which handler, which streaming type?)

## Conclusion

This compatibility architecture ensures the turul-mcp-framework works seamlessly with the current MCP client ecosystem while maintaining full specification compliance. The solution prioritizes developer experience and real-world usability over strict specification enforcement, with clear paths forward as the ecosystem matures.

The architecture successfully resolves the MCP Inspector timeout issues while preserving all advanced MCP 2025-06-18 Streamable HTTP capabilities for compliant clients.