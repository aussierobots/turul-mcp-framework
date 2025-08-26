# MCP Framework Streamable HTTP Transport Architecture

## Executive Summary

The MCP Framework implements the **MCP 2025-06-18 Streamable HTTP Transport** specification with a complete session architecture and pluggable backends. The architecture consists of two complementary components that together implement the full Streamable HTTP protocol:

1. **JSON-RPC Handler**: `mcp-server::SessionManager` (handles POST requests and session state)
2. **SSE Stream Handler**: `SessionStorage` + `StreamManager` (handles SSE streaming and notifications)

This document provides the definitive reference for understanding and connecting these systems.

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
│   • All 7 MCP notification types supported                               │
├─────────────────────────────────────────────────────────────────────────┤
│ Implementation:                                                           │
│   ✅ StreamManagerNotificationBroadcaster (bridges to StreamManager)     │
│   🔜 NatsNotificationBroadcaster (distributed)                           │
│   🔜 SnsNotificationBroadcaster (AWS fan-out)                            │
└─────────────────────────────────────────────────────────────────────────┘
```

## 🔴 The Issue: Disconnected Streamable HTTP Components

### Component 1: JSON-RPC Handler (mcp-server crate)
```
Location: crates/mcp-server/src/session.rs
Purpose: Handles POST JSON-RPC requests and session state per MCP Streamable HTTP spec

Flow:
Tool.call(SessionContext) 
    ↓
SessionContext.notify_log() [Line 171-184]
    ↓
self.notify(SessionEvent::Notification(json_value)) [Line 183]
    ↓
(self.send_notification)(event) [Line 132]
    ↓
NotificationBroadcaster → StreamManager bridge
    ↓
✅ Working JSON-RPC processing (POST requests)
```

### Component 2: SSE Stream Handler (http-mcp-server crate)
```
Location: crates/http-mcp-server/src/stream_manager.rs
Purpose: Handles SSE streams with resumability per MCP Streamable HTTP spec

Flow:
Client GET /mcp (Accept: text/event-stream)
    ↓
SessionMcpHandler.handle_mcp_request() 
    ↓
StreamManager.handle_sse_connection() [Line 113]
    ↓
Creates broadcast channel [Line 29: HashMap<String, broadcast::Sender>]
    ↓
✅ Working SSE stream infrastructure (GET requests)
```

### Why They're Disconnected

1. **Different Broadcast Channels**: Each component creates its own tokio broadcast channels
2. **Different Session Contexts**: Tools get `mcp-server::SessionContext`, SSE uses `StreamManager`
3. **Missing Bridge**: NotificationBroadcaster exists but wasn't fully wired to complete the Streamable HTTP flow

## ✅ What's Already Implemented

### SessionStorage Trait (Complete)
- **Location**: `crates/mcp-session-storage/src/traits.rs`
- **Status**: ✅ Fully implemented with 30+ methods
- **Features**:
  - Session lifecycle (create, update, delete)
  - Stream management (per-session SSE streams)
  - Event persistence (with monotonic IDs)
  - State management (key-value per session)
  - Cleanup and maintenance

### StreamManager (Complete)
- **Location**: `crates/http-mcp-server/src/stream_manager.rs`
- **Status**: ✅ Fully implemented with resumability
- **Features**:
  - Per-session broadcast channels
  - Last-Event-ID support for reconnection
  - Event replay from storage
  - Proper SSE formatting
  - Keep-alive and timeout handling

### NotificationBroadcaster (Complete)
- **Location**: `crates/http-mcp-server/src/notification_bridge.rs`
- **Status**: ✅ All MCP notification types supported
- **Features**:
  - 7 notification types (progress, message, resources, tools, prompts, cancelled)
  - JSON-RPC compliant formatting
  - StreamManager bridge implementation
  - Ready for distributed backends

## 🔧 The Fix: Connect the Systems

### Step 1: Update SessionContext Creation
**File**: `crates/mcp-server/src/server.rs` (Line ~575)

Current:
```rust
SessionContext::from_json_rpc_session(json_rpc_ctx, self.session_manager.clone())
```

Fix:
```rust
SessionContext::from_json_rpc_with_broadcaster(json_rpc_ctx)
// Broadcaster is already in json_rpc_ctx from http-mcp-server
```

### Step 2: Fix Notification Methods
**File**: `crates/mcp-server/src/session.rs` (Line ~171)

Current:
```rust
pub fn notify_log(&self, level: &str, message: impl Into<String>) {
    // Creates SessionEvent and sends to OLD SessionManager
    self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
}
```

Fix:
```rust
pub fn notify_log(&self, level: &str, message: impl Into<String>) {
    // Extract broadcaster from context and use directly
    if let Some(broadcaster) = self.get_broadcaster() {
        broadcaster.send_message_notification(
            &self.session_id,
            LoggingMessageNotification { ... }
        ).await;
    }
}
```

### Step 3: Ensure Broadcaster is Passed
**File**: `crates/http-mcp-server/src/session_handler.rs` (Line ~276)

Status: ✅ Already done! Broadcaster is passed in SessionContext.

## 🔮 Future Architecture Extensions

### Distributed Notifications (SNS/NATS)
```rust
// Current: Single-instance
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
- **Status**: ✅ Implemented

### ADR-002: Per-Session Broadcast Channels
- **Decision**: Each session has its own broadcast channel
- **Rationale**: Prevents cross-talk, enables targeted delivery
- **Status**: ✅ Implemented

### ADR-003: Monotonic Event IDs
- **Decision**: Global atomic counter for event IDs
- **Rationale**: Guarantees ordering, enables resumability
- **Status**: ✅ Implemented

### ADR-004: Storage Abstraction First
- **Decision**: Build trait abstraction before implementations
- **Rationale**: Ensures all backends have same interface
- **Status**: ✅ Trait complete, implementations pending

## 🚨 Critical Implementation Notes

1. **DO NOT** create new session systems - connect existing ones
2. **DO NOT** modify SessionStorage trait - it's complete
3. **DO NOT** change StreamManager - it works correctly
4. **DO** wire NotificationBroadcaster to tools
5. **DO** remove old SessionManager notification code
6. **DO** test end-to-end flow after connection

## 📋 Testing the Connection

### Test 1: Basic Notification Flow
```bash
# Start server
cargo run --example client-initialise-server

# In another terminal, run client
cargo run --example client-initialise-report

# Expected: Tool notifications appear in SSE stream
# Current: "channel closed" error
```

### Test 2: Session Isolation
```bash
# Create two sessions
# Send notification from tool in session 1
# Verify only session 1 SSE receives it
# Verify session 2 SSE doesn't receive it
```

### Test 3: SSE Resumability
```bash
# Connect SSE with Last-Event-ID: 100
# Verify events 101+ are replayed
# Verify real-time events continue
```

## 📚 References

- **SessionStorage Trait**: `crates/mcp-session-storage/src/traits.rs`
- **StreamManager**: `crates/http-mcp-server/src/stream_manager.rs`
- **NotificationBroadcaster**: `crates/http-mcp-server/src/notification_bridge.rs`
- **OLD SessionManager**: `crates/mcp-server/src/session.rs` (to be disconnected)
- **MCP Specification**: TypeScript schema for notification format