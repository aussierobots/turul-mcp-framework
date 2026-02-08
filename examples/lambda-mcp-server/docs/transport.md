# MCP Streamable HTTP Transport Architecture

## Overview

This document describes the **MCP 2025-11-25 Streamable HTTP transport** implementation in the lambda-turul-mcp-server, including the notification system architecture and current implementation status.

## MCP 2025-11-25 Compliance

### Supported HTTP Methods

#### ✅ POST /mcp - JSON-RPC Messages
```http
POST /mcp HTTP/1.1
Content-Type: application/json
Accept: application/json, text/event-stream
mcp-session-id: session-12345

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {"tools": {}, "resources": {}}
  }
}
```

**Supported Methods:**
- `initialize` - Session initialization with proper header management
- `tools/list` - List available tools
- `tools/call` - Execute tools with session context
- `notifications/initialized` - Client initialization complete
- `ping` - Server health check

#### ✅ GET /mcp - Server-Sent Events Stream
```http
GET /mcp HTTP/1.1
Accept: text/event-stream
mcp-session-id: session-12345
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
mcp-session-id: session-12345

data: {"type": "connection", "status": "connected", "session_id": "session-12345"}

```

#### ✅ OPTIONS /mcp - CORS Preflight
```http
OPTIONS /mcp HTTP/1.1
Origin: https://example.com
```

**Response:**
```http
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Accept, mcp-session-id
```

## Notification System Architecture

### Two-Tier Event System

Our implementation uses a clean separation between internal and external events:

```
┌─────────────────────────────────────────────────────────────┐
│                    Lambda MCP Server                       │
│                                                             │
│  ┌─────────────────┐    ┌─────────────────────────────────┐  │
│  │ Internal Events │───▶│ tokio::broadcast channel        │  │
│  │ • Tool calls    │    │ (GLOBAL_EVENT_CHANNEL)          │  │
│  │ • Server health │    │ - Capacity: 1000 events        │  │
│  │ • Session mgmt  │    │ - OnceCell initialization      │  │
│  └─────────────────┘    └─────────────────────────────────┘  │
│                                     │                       │
│  ┌─────────────────┐                │                       │
│  │ SNS Processor   │───────────────▶│                       │
│  │ (external       │                │                       │
│  │  event trigger) │                │                       │
│  └─────────────────┘                │                       │
│           ▲                         │                       │
│           │                         ▼                       │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │ SSE Connections (subscribe to tokio broadcast)         │  │
│  │ • GET /mcp endpoints                                   │  │
│  │ • Real-time event streaming                            │  │
│  │ • Session-filtered events                              │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                          ▲
                          │
              ┌─────────────────────┐
              │ SNS Topic           │
              │ (optional)          │
              └─────────────────────┘
                          ▲
                          │
              ┌─────────────────────┐
              │ External Systems    │
              │ • CloudWatch        │
              │ • Other services    │
              └─────────────────────┘
```

### Event Types

#### Internal Events (tokio::broadcast)
```rust
pub enum GlobalEvent {
    SystemHealth {
        component: String,
        status: String,
        details: Value,
        timestamp: DateTime<Utc>,
    },
    ToolExecution {
        tool_name: String,
        session_id: String,
        status: ToolExecutionStatus,
        result: Option<Value>,
        timestamp: DateTime<Utc>,
    },
    SessionUpdate {
        session_id: String,
        event_type: SessionEventType,
        data: Option<Value>,
        timestamp: DateTime<Utc>,
    },
    MonitoringUpdate {
        resource_type: String,
        region: String,
        correlation_id: String,
        data: Value,
        timestamp: DateTime<Utc>,
    },
}
```

#### Broadcasting Functions
```rust
// Available in global_events.rs
pub async fn broadcast_global_event(event: GlobalEvent) -> Result<usize, SendError<GlobalEvent>>
pub async fn broadcast_tool_progress(tool_name: impl Into<String>, session_id: impl Into<String>, status: ToolExecutionStatus, result: Option<Value>) -> Result<usize, SendError<GlobalEvent>>
pub async fn broadcast_system_health(component: impl Into<String>, status: impl Into<String>, details: Value) -> Result<usize, SendError<GlobalEvent>>
pub async fn broadcast_session_event(session_id: impl Into<String>, event_type: SessionEventType, data: Option<Value>) -> Result<usize, SendError<GlobalEvent>>
pub async fn broadcast_monitoring_update(resource_type: impl Into<String>, region: impl Into<String>, correlation_id: impl Into<String>, data: Value) -> Result<usize, SendError<GlobalEvent>>
```

## Current Implementation Status

### ✅ What's Working
1. **HTTP Transport Layer**
   - POST /mcp JSON-RPC endpoint fully functional
   - GET /mcp SSE endpoint responds with proper headers
   - OPTIONS CORS preflight working
   - Session ID header management

2. **tokio Broadcast Infrastructure**
   - Global event channel initialized with OnceCell
   - Event types and serialization complete
   - Broadcasting functions available
   - 1000-event capacity configured

3. **Session Management**
   - DynamoDB-backed session persistence
   - Session creation, validation, and lifecycle
   - TTL-based cleanup
   - OnceCell pattern for global state

### ⚠️ Current Limitations
1. **SSE Event Streaming**
   - SSE endpoint returns static connection message
   - **Not connected to tokio broadcast channel**
   - No active subscribers causing `SendError` logs

2. **Tool Result Handling**
   - Tools execute successfully but results not returned
   - Generic "Tool execution completed" instead of actual output
   - Event broadcasting fails due to no subscribers

3. **Session Validation**
   - Tools run with "unknown" session IDs
   - Missing session header validation in tool execution
   - No proper error handling for missing session context

## Required Fixes for Full Functionality

### 1. Connect SSE to tokio Broadcast
```rust
// In handle_mcp_get_request, need to:
let mut receiver = subscribe_to_global_events().ok_or(...)?;
// Stream events from receiver to SSE response
```

### 2. Fix Tool Result Handling
```rust
// In handle_tool_call, return actual tool results:
Ok(json!({
    "jsonrpc": "2.0",
    "result": {
        "content": extract_tool_content(result),  // Not generic message
        "isError": result.is_error
    }
}))
```

### 3. Implement Session Validation
```rust
// Validate session ID before tool execution:
let session_id = extract_session_id(&headers)
    .ok_or_else(|| create_mcp_error(-32602, "Missing mcp-session-id header"))?;
```

## External Event Integration

### SNS Topic Configuration
```bash
# Optional SNS topic for external events
export SNS_TOPIC_ARN="arn:aws:sns:us-east-1:123456789012:mcp-global-events"
```

### External Event Flow
1. External system publishes to SNS topic
2. SNS triggers Lambda function
3. Lambda converts SNS message to GlobalEvent
4. Event broadcasted via tokio channel
5. All connected SSE streams receive event

### Example External Event
```bash
aws sns publish \
  --topic-arn "$SNS_TOPIC_ARN" \
  --subject "aws.ec2.state_change" \
  --message '{
    "source": "aws.ec2",
    "event_type": "resource_update",
    "data": {
      "resource_id": "i-1234567890abcdef0",
      "previous_state": "pending",
      "current_state": "running"
    }
  }'
```

## Performance Characteristics

### tokio Broadcast Benefits
- **Non-blocking**: Events distributed without blocking sender
- **Fan-out**: Single event reaches all subscribers simultaneously
- **Memory efficient**: Circular buffer with configurable capacity
- **No message competition**: Unlike SQS, all subscribers get all events

### Lambda Scaling
- **Single instance**: Multiple SSE connections share same tokio broadcast
- **Multiple instances**: External SNS events trigger all instances
- **Session affinity**: API Gateway can route related requests to same instance

## Testing the Transport

### Basic JSON-RPC Testing
```bash
curl -X POST http://127.0.0.1:9000/mcp \
  -H "Content-Type: application/json" \
  -H "mcp-session-id: test-session" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
```

### SSE Stream Testing
```bash
curl -X GET http://127.0.0.1:9000/mcp \
  -H "Accept: text/event-stream" \
  -H "mcp-session-id: test-session"
```

### Multi-client Testing
```bash
# Test concurrent SSE connections
cd ../lambda-mcp-client
cargo run -- test --url http://127.0.0.1:9000 --suite streaming --concurrency 3
```

## Future Enhancements

1. **Complete SSE Integration**: Connect GET /mcp to tokio broadcast
2. **Rich Tool Results**: Return actual tool output with content types
3. **Event Filtering**: Session-specific event filtering for SSE streams
4. **Subscription Management**: Dynamic event type subscription
5. **Backpressure Handling**: Handle slow SSE consumers
6. **Metrics and Monitoring**: Event throughput and subscription analytics

## Implementation Notes

- **OnceCell Pattern**: Global state initialized once, reused across requests
- **Error Handling**: Proper JSON-RPC error responses for protocol violations
- **CORS Support**: Full cross-origin support for web clients
- **Session Isolation**: Events can be filtered by session ID
- **Graceful Degradation**: Server works without SNS topic (internal events only)

This architecture provides a solid foundation for real-time MCP notifications while maintaining the simplicity and scalability benefits of serverless Lambda deployment.