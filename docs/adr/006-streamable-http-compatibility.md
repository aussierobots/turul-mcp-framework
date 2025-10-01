# ADR-006: MCP Streamable HTTP Transport Compatibility Architecture

**Status**: Accepted  
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
| `application/json, text/event-stream` | Compliant | Server choice | Full MCP 2025-06-18 compliance |
| `application/json` | JsonOnly | No | MCP Inspector compatibility |
| `text/event-stream` | SseOnly | Yes* | Edge case support |
| `*/*` | JsonOnly | No | Treated as JSON-only for safety |
| Invalid/Empty | Invalid | No | Fallback to JSON |

*Subject to server `enable_sse` configuration

### Configuration Precedence

1. **Server Configuration**: `enable_sse=false` disables all SSE usage
2. **Accept Header Compliance**: Non-compliant headers force JSON responses  
3. **Method Type**: Only `tools/call` methods use POST SSE (when conditions met)
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

### Client Ecosystem Maturity
- As clients become more compliant, JsonOnly fallback usage should decrease
- Server operators can monitor compliance via debug logs
- Migration path exists to stricter compliance enforcement

### Lambda Streaming Future
- AWS Lambda may add support for persistent connections/streaming responses
- If AWS adds this capability, server-initiated notifications may work without code changes
- Monitor AWS Lambda roadmap for streaming response support

## Conclusion

This compatibility architecture ensures the turul-mcp-framework works seamlessly with the current MCP client ecosystem while maintaining full specification compliance. The solution prioritizes developer experience and real-world usability over strict specification enforcement, with clear paths forward as the ecosystem matures.

The architecture successfully resolves the MCP Inspector timeout issues while preserving all advanced MCP 2025-06-18 Streamable HTTP capabilities for compliant clients.