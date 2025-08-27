# MCP Framework Streamable HTTP Transport Architecture

## Executive Summary

The MCP Framework implements the **MCP 2025-06-18 Streamable HTTP Transport** specification with a complete session architecture and pluggable backends. The architecture consists of two complementary components that together implement the full Streamable HTTP protocol:

1. **JSON-RPC Handler**: `SessionMcpHandler` (handles POST requests and session state)
2. **SSE Stream Handler**: `SessionStorage` + `StreamManager` (handles SSE streaming and notifications)

**Status**: ✅ **FULLY WORKING** - Complete end-to-end notification delivery from tools to SSE streams.

## 🏗️ Complete Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          SESSION STORAGE LAYER                           │
│                     (Pluggable Backend Abstraction)                      │
├─────────────────────────────────────────────────────────────────────────┤
│ SessionStorage Trait (mcp-session-storage/src/traits.rs)                 │
│   • create_session() → UUID v7 (temporal ordering)                       │
│   • store_event() → Monotonic event IDs                                  │
│   • get_events_after() → SSE resumability support                        │
│   • 30+ methods for complete session lifecycle                           │
├─────────────────────────────────────────────────────────────────────────┤
│ Implementations:                                                          │
│   ✅ InMemorySessionStorage (complete, working)                          │
│   🔜 SqliteSessionStorage (trait ready, impl pending)                    │
│   🔜 PostgresSessionStorage (trait ready, impl pending)                  │
│   🔜 DynamoDbSessionStorage (trait ready, impl pending)                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                           STREAM MANAGER LAYER                           │
│                        (SSE Streaming with Channels)                     │
├─────────────────────────────────────────────────────────────────────────┤
│ StreamManager<S: SessionStorage> (http-mcp-server/src/stream_manager.rs) │
│   • HashMap<SessionId, broadcast::Sender<SseEvent>>                      │
│   • handle_sse_connection() with Last-Event-ID support                   │
│   • broadcast_to_session() for targeted event delivery                   │
│   • create_post_sse_stream() for POST SSE responses                      │
│   • Event replay from SessionStorage on reconnect                        │
│   • Per-session isolation (no cross-talk)                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                       NOTIFICATION BROADCASTER LAYER                     │
│                    (MCP Protocol Notification Routing)                   │
├─────────────────────────────────────────────────────────────────────────┤
│ NotificationBroadcaster Trait (http-mcp-server/src/notification_bridge)  │
│   • send_progress_notification() - MCP compliant                         │
│   • send_message_notification() - MCP compliant                          │
│   • send_resource_updated_notification() - MCP compliant                 │
│   • All 6 MCP notification types supported                               │
├─────────────────────────────────────────────────────────────────────────┤
│ Implementation:                                                           │
│   ✅ StreamManagerNotificationBroadcaster (bridges to StreamManager)     │
│   🔜 NatsNotificationBroadcaster (distributed)                           │
│   🔜 SnsNotificationBroadcaster (AWS fan-out)                            │
└─────────────────────────────────────────────────────────────────────────┘
```

## ✅ Current Implementation: Connected Streamable HTTP Components

### Component 1: JSON-RPC Handler (SessionMcpHandler)
```
Location: crates/http-mcp-server/src/session_handler.rs
Purpose: Handles POST JSON-RPC requests and session state per MCP Streamable HTTP spec

Flow:
Tool.call(SessionContext) 
    ↓
SessionContext.notify_log() / notify_progress()
    ↓
NotificationBroadcaster.send_notification()
    ↓
StreamManager.broadcast_to_session()
    ↓
✅ Working JSON-RPC processing with notification delivery
```

### Component 2: SSE Stream Handler (StreamManager)
```
Location: crates/http-mcp-server/src/stream_manager.rs
Purpose: Handles SSE streams with resumability per MCP Streamable HTTP spec

Flow:
Client POST /mcp (Accept: text/event-stream)
    ↓
SessionMcpHandler.create_post_sse_response()
    ↓
StreamManager.create_post_sse_stream()
    ↓
Creates broadcast channel + event replay
    ↓
✅ Working SSE stream infrastructure with notification delivery
```

### MCP Streamable HTTP Protocol Implementation

**POST Requests with SSE Response**:
- Client sends `POST /mcp` with `Accept: text/event-stream` header
- Server processes JSON-RPC request and returns SSE stream
- Tool notifications appear in the same POST SSE response
- Each POST request creates an isolated SSE stream for that request

**GET Requests for Persistent Streams**:
- Client sends `GET /mcp` with `Accept: text/event-stream` header  
- Server creates persistent SSE connection for server-initiated events
- Supports Last-Event-ID for resumability

## ✅ Fully Implemented Components

### SessionStorage Trait (Complete)
- **Location**: `crates/mcp-session-storage/src/traits.rs`
- **Status**: ✅ Fully implemented with 30+ methods
- **Features**:
  - Session lifecycle (create, update, delete)
  - Event persistence (with monotonic IDs for resumability)
  - State management (key-value per session)
  - Cleanup and maintenance

### StreamManager (Complete & Connected)
- **Location**: `crates/http-mcp-server/src/stream_manager.rs`
- **Status**: ✅ Fully implemented with resumability
- **Features**:
  - Per-session broadcast channels
  - Last-Event-ID support for reconnection
  - Event replay from storage
  - Proper SSE formatting
  - POST SSE stream creation for tool notifications

### NotificationBroadcaster (Complete & Connected)
- **Location**: `crates/http-mcp-server/src/notification_bridge.rs`
- **Status**: ✅ All MCP notification types supported and connected
- **Features**:
  - 6 notification types (progress, message, cancelled, resources, tools)
  - JSON-RPC compliant formatting
  - StreamManager bridge implementation
  - End-to-end delivery confirmed

## ✅ Connected Systems - Current Implementation

### Session Context Integration (Implemented)
**File**: `crates/http-mcp-server/src/session_handler.rs`

```rust
// SessionMcpHandler creates SessionContext with NotificationBroadcaster
let session_context = SessionContext {
    session_id: session_id.unwrap_or_default(),
    broadcaster: Some(self.notification_broadcaster.clone()),
    // ... other fields
};

// Tools receive SessionContext and can send notifications
tool.call(session_context).await
```

### Notification Flow (Implemented)
**File**: `crates/mcp-server/src/session.rs`

```rust
// SessionContext provides notification methods
pub fn notify_log(&self, level: &str, message: impl Into<String>) {
    if let Some(broadcaster) = &self.broadcaster {
        // Creates proper JSON-RPC notification
        let notification = JsonRpcNotification::new_with_object_params(
            "notifications/message".to_string(),
            params_map
        );
        // Async bridge sends to StreamManager
        tokio::spawn(async move {
            broadcaster.send_notification(session_id, notification).await
        });
    }
}
```

### SSE Response Integration (Implemented)
**File**: `crates/http-mcp-server/src/stream_manager.rs`

```rust
// POST requests with Accept: text/event-stream return SSE streams
pub async fn create_post_sse_stream(&self, 
    response: JsonRpcResponse, 
    session_id: String
) -> Result<Response<JsonRpcBody>> {
    // Creates streaming response with tool result + notifications
    // Events stored with monotonic IDs for resumability
    // Returns proper SSE formatted response
}
```

## 🔮 Future Architecture Extensions

### Distributed Notifications (SNS/NATS)
```rust
// Current: Single-instance (Working)
StreamManager → tokio::broadcast → Local SSE clients

// Future: Multi-instance
StreamManager → NotificationBroadcaster →
    ├── tokio::broadcast (local clients)
    ├── NATS JetStream (other instances)
    └── AWS SNS (Lambda functions)
```

### Additional Storage Backends
```rust
// All implement same SessionStorage trait
PostgresSessionStorage → Production databases
DynamoDbSessionStorage → Serverless/Lambda
RedisSessionStorage → Cache layer
S3SessionStorage → Long-term event archive
```

## 📊 Architecture Decision Records

### ADR-001: UUID v7 for Session IDs
- **Decision**: Use UUID v7 (not v4) for all session IDs
- **Rationale**: Provides temporal ordering, better for databases
- **Status**: ✅ Implemented and working

### ADR-002: Per-Session Broadcast Channels
- **Decision**: Each session has its own broadcast channel
- **Rationale**: Prevents cross-talk, enables targeted delivery
- **Status**: ✅ Implemented and working

### ADR-003: Monotonic Event IDs
- **Decision**: Global atomic counter for event IDs
- **Rationale**: Guarantees ordering, enables resumability
- **Status**: ✅ Implemented and working

### ADR-004: POST SSE Response Pattern
- **Decision**: POST requests with Accept: text/event-stream return SSE streams
- **Rationale**: MCP Streamable HTTP specification compliance
- **Status**: ✅ Implemented and working

### ADR-005: Storage Abstraction First
- **Decision**: Build trait abstraction before implementations
- **Rationale**: Ensures all backends have same interface
- **Status**: ✅ Trait complete, InMemory implementation working

## ✅ Testing the Connected System

### Test 1: Basic Notification Flow (PASSING)
```bash
# Start server
cargo run --example client-initialise-server -- --port 52935

# Test complete notification flow
export RUST_LOG=debug
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp

# Result: "🎆 FULLY MCP COMPLIANT: Session management + Streamable HTTP working!"
```

### Test 2: Session Management (WORKING)
- ✅ Server creates UUID v7 sessions during initialize
- ✅ Session IDs returned via `Mcp-Session-Id` header
- ✅ Client uses session ID in subsequent requests
- ✅ Session isolation prevents cross-talk

### Test 3: SSE Resumability (IMPLEMENTED)
- ✅ Last-Event-ID header processing
- ✅ Event replay from storage
- ✅ Monotonic event IDs for proper ordering
- ✅ Real-time event continuation after replay

## 📚 References

- **SessionStorage Trait**: `crates/mcp-session-storage/src/traits.rs`
- **StreamManager**: `crates/http-mcp-server/src/stream_manager.rs`
- **NotificationBroadcaster**: `crates/http-mcp-server/src/notification_bridge.rs`
- **SessionMcpHandler**: `crates/http-mcp-server/src/session_handler.rs`
- **Working Test Example**: `crates/mcp-server/examples/client-initialise-report.rs`
- **MCP Specification**: Complete MCP 2025-06-18 Streamable HTTP Transport implementation