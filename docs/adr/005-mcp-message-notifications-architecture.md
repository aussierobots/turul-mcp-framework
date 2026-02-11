# ADR-005: MCP Message Notifications and SSE Streaming Architecture

**Status**: Active  
**Date**: 2025-09-02  
**Drivers**: Nick Aversano, Claude Code

## Context

The turul-mcp-framework implements MCP (Model Context Protocol) 2025-06-18 with complete Streamable HTTP support. However, an issue was discovered where MCP Inspector and other clients are not receiving notifications through POST SSE responses due to incorrect event type formatting in the Server-Sent Events stream.

### Current Architecture

The framework supports dual-path notification delivery:

1. **GET SSE** - Persistent Server-Sent Events stream (`GET /mcp` with `Accept: text/event-stream`)
2. **POST SSE** - Tool execution with streaming response (`POST /mcp` with `Accept: text/event-stream`)

### The Problem

POST SSE responses currently use hardcoded `event: data` instead of proper MCP event types:

```
// Current (BROKEN):
event: data
data: {"jsonrpc":"2.0","method":"notifications/message","params":{...}}

// Expected (CORRECT):
event: notifications/message  
data: {"jsonrpc":"2.0","method":"notifications/message","params":{...}}
```

This causes MCP Inspector and other clients to not recognize or display the notifications correctly.

## Decision

We will implement a **dual-stream notification architecture** with proper SSE event type formatting for complete MCP Streamable HTTP compliance.

### Notification Flow Architecture

```
Tool Execution
    ↓
SessionContext.notify_log()
    ↓
NotificationBroadcaster.send_message_notification()
    ↓
StreamManager.broadcast_to_session()
    ↓
┌─────────────────┬─────────────────┐
│   GET SSE       │   POST SSE      │
│ (Persistent)    │ (Tool Response) │
└─────────────────┴─────────────────┘
           ↓               ↓
    event: notifications/message
    data: {...JSON-RPC...}
```

### SSE Event Type Mapping

All MCP notifications must use their proper method name as the SSE event type:

| MCP Method | SSE Event Type |
|------------|----------------|
| `notifications/message` | `event: notifications/message` |
| `notifications/progress` | `event: notifications/progress` |  
| `notifications/cancelled` | `event: notifications/cancelled` |
| `notifications/initialized` | `event: notifications/initialized` |
| `notifications/resources/updated` | `event: notifications/resources/updated` |
| `notifications/resources/list_changed` | `event: notifications/resources/list_changed` |
| `notifications/tools/list_changed` | `event: notifications/tools/list_changed` |
| `notifications/prompts/list_changed` | `event: notifications/prompts/list_changed` |
| `notifications/roots/list_changed` | `event: notifications/roots/list_changed` |

### Message Notification Architecture

Message notifications (logging) follow this specific pattern:

#### SessionContext Integration
```rust
// Session-aware logging with filtering
session.notify_log(
    LoggingLevel::Info,                    // Level for filtering
    serde_json::json!("Log message"),     // Message content
    Some("component".to_string()),        // Logger name (optional)
    Some(correlation_map)                 // Meta with correlation_id (optional)
).await;
```

#### NotificationBroadcaster Processing
```rust  
// Convert to MCP LoggingMessageNotification
let notification = LoggingMessageNotification {
    method: "notifications/message".to_string(),
    params: LoggingMessageParams {
        level: LoggingLevel::Info,
        data: serde_json::json!("Log message"), 
        logger: Some("component".to_string()),
        meta: Some(correlation_map),
    }
};

// Send via StreamManager with proper event type
stream_manager.broadcast_to_session(
    session_id,
    notification.method.clone(), // "notifications/message" 
    serde_json::to_value(&notification)?
).await?;
```

#### SSE Stream Formatting
```rust
// GET SSE (persistent stream)
event: notifications/message
id: 42
data: {"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info","data":"Log message","logger":"component","_meta":{"correlation_id":"uuid-v7"}}}

// POST SSE (tool response stream)  
event: notifications/message
data: {"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info","data":"Log message","logger":"component","_meta":{"correlation_id":"uuid-v7"}}}

event: result
data: {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Tool executed successfully"}]}}
```

## Implementation

### Phase 1: Fix StreamManager Event Formatting

**File**: `crates/turul-http-mcp-server/src/stream_manager.rs`

**Current (lines 490-492)**:
```rust
let notification_sse = format!(
    "event: data\ndata: {}\n\n",
    event.data
);
```

**Fixed**:
```rust  
let notification_sse = format!(
    "event: {}\ndata: {}\n\n",
    event.event_type,  // Use actual event type (e.g., "notifications/message")
    event.data
);
```

**Current (lines 501-503)**:
```rust
let response_sse = format!(
    "event: data\ndata: {}\n\n", 
    response_json
);
```

**Fixed**:
```rust
let response_sse = format!(
    "event: result\ndata: {}\n\n",  // Tool responses use "result" event type
    response_json  
);
```

### Phase 2: Re-enable Streamable HTTP

**File**: `crates/turul-http-mcp-server/src/session_handler.rs`

**Remove temporary compatibility fix (lines 395-398)**:
```rust
// TEMPORARY FIX: Disable MCP Streamable HTTP for tool calls to ensure MCP Inspector compatibility
// Always return JSON responses for all operations until SSE implementation is fixed
debug!("🔧 COMPATIBILITY MODE: Always returning JSON response for method: {:?} (SSE disabled for tool calls)", method_name);
Ok(jsonrpc_response_with_session(response, response_session_id)?)
```

**Replace with conditional SSE logic**:
```rust
if is_tool_call && accepts_sse {
    debug!("📡 Creating POST SSE stream for tool call with notifications");
    match self.stream_manager.create_post_sse_stream(
        response_session_id.clone().unwrap_or_default(),
        response,
    ).await {
        Ok(sse_response) => Ok(sse_response),
        Err(e) => {
            warn!("Failed to create POST SSE stream, falling back to JSON: {}", e);
            Ok(jsonrpc_response_with_session(response, response_session_id)?)
        }
    }
} else {
    Ok(jsonrpc_response_with_session(response, response_session_id)?)
}
```

### Phase 3: Test Framework Updates

**Add command-line switches for testing both modes**:

- `logging-test-server --enable-post-sse` - Enable POST SSE responses
- `logging-test-client --test-post-sse --test-get-sse` - Test both streaming modes

## Consequences

### Positive
- **MCP Inspector Compatibility**: Notifications will appear correctly in MCP Inspector
- **Complete Streamable HTTP Compliance**: Both POST and GET SSE patterns working
- **Proper Event Semantics**: SSE event types match MCP method names exactly
- **Dual-Stream Testing**: Comprehensive test coverage for both streaming modes
- **Correlation ID Tracking**: Works consistently across both POST and GET SSE

### Negative  
- **Temporary Compatibility**: May temporarily break clients expecting hardcoded `event: data`
- **Testing Complexity**: Need to verify both streaming modes work correctly
- **Implementation Risk**: Changes to critical streaming paths require careful testing

### Mitigation Strategies
- **Gradual Rollout**: Implement behind feature flags initially  
- **Comprehensive Testing**: Test with both MCP Inspector and curl
- **Fallback Logic**: Maintain JSON-only fallback for compatibility
- **Client Verification**: Test multiple MCP clients to ensure broad compatibility

## Related Decisions

- **ADR-001**: Session Storage Architecture - Session management for SSE connections
- **ADR-003**: JsonSchema Standardization - Type-safe notification serialization  
- **ADR-004**: SessionContext Macro Support - Tool integration with notification system

## References

- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/)
- [Server-Sent Events Standard](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)