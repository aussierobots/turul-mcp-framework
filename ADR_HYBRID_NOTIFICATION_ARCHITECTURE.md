# Architecture Decision Record: Single StreamManager with Internal Session Management  

**Date**: 2025-08-27  
**Status**: Implementing (Revised)  
**Context**: Critical SSE notification delivery issue (Day 3+ blocking)

**Revision Note**: Based on GEMINI analysis, simplified from complex "GlobalStreamManagerRegistry" to elegant single StreamManager approach.

## Problem

The MCP framework's SSE notification delivery system was broken due to architectural confusion between:

1. **Session Isolation** (MCP requirement): Each session should be independent
2. **Server-wide Broadcasting** (operational requirement): Ability to send notifications to ALL sessions

### Failed Approaches

#### Approach 1: Shared StreamManager (FAILED)
```rust
// ❌ WRONG: Single StreamManager shared across ALL sessions
let shared_manager = Arc::new(StreamManager::new());
// Problem: Violated session isolation, cross-session data leaks
```

#### Approach 2: Pure Per-Session (FAILED) 
```rust
// ❌ WRONG: Completely isolated per-session managers
let manager = StreamManager::new(); // Created per HTTP connection
// Problem: No way to broadcast to all sessions, managers not reused
```

## Decision

Implement **Single StreamManager with Internal Session Management**

### Core Architecture

```rust
/// Single StreamManager that internally manages per-session channels
pub struct StreamManager<S: SessionStorage> {
    storage: Arc<S>,
    // ✅ Session isolation via internal HashMap keying by session_id
    connections: Arc<RwLock<HashMap<String, HashMap<ConnectionId, mpsc::Sender<SseEvent>>>>>,
    config: StreamConfig,
}

impl<S: SessionStorage> StreamManager<S> {
    /// Send to specific session (session isolation)
    pub async fn broadcast_to_session(&self, session_id: &str, event_type: String, data: Value) 
        -> Result<u64, StreamError>;
    
    /// Send to ALL sessions (server-wide broadcasts)  
    pub async fn broadcast_to_all_sessions(&self, event_type: String, data: Value)
        -> Result<Vec<String>, StreamError>;
}
```

### Server Integration

```rust
// Single shared StreamManager at server level
pub struct HttpMcpServer<S: SessionStorage> {
    stream_manager: Arc<StreamManager<S>>, // ✅ Shared across all connections
}

// All HTTP connections use the same StreamManager instance
pub struct SessionMcpHandler<S: SessionStorage> {
    stream_manager: Arc<StreamManager<S>>, // ✅ Same instance for all handlers
}
```

### Architecture Flow

```
HTTP Server
    ↓
HttpMcpServer {
    stream_manager: Arc<StreamManager<S>>,  // ✅ Single shared instance
}
    ↓ (passes to each connection)
SessionMcpHandler {
    stream_manager: Arc<StreamManager<S>>,  // ✅ Same instance
}
    ↓
StreamManager {
    connections: HashMap<SessionId, HashMap<ConnectionId, Sender>> {
        "session_1": { "conn_1": sender, "conn_2": sender },  // Session isolation
        "session_2": { "conn_3": sender },                    // Session isolation
        "session_3": { "conn_4": sender, "conn_5": sender },  // Session isolation
    }
}
```

### Notification Types

| Notification Type | Method | Scope | Use Cases |
|-------------------|--------|-------|-----------|
| **Session-specific** | `stream_manager.broadcast_to_session()` | Single session | Tool results, progress updates |
| **Server-wide** | `stream_manager.broadcast_to_all_sessions()` | All sessions | System logs, server status |

## Benefits

### ✅ Session Isolation
- Internal HashMap keying by session_id provides complete session separation
- No cross-session data contamination  
- MCP specification compliance

### ✅ Global Coordination  
- Single StreamManager can iterate all sessions for server-wide broadcasts
- System-wide notifications work correctly
- Operational tooling support

### ✅ Resource Efficiency
- Single StreamManager instance shared across all HTTP connections
- No complex registry management or instance caching needed
- Automatic cleanup when sessions expire

### ✅ MCP Compliance
- Maintains "one connection per session" requirement
- Proper session isolation via internal channel management
- No broadcast violations (each session gets individual copy)

### ✅ Architectural Simplicity
- No complex GlobalStreamManagerRegistry pattern
- Single shared instance with internal session management
- Elegant solution that's easy to understand and maintain

## Implementation Strategy

### Phase 1: StreamManager Enhancement (⏳ PENDING)
- [ ] Add `broadcast_to_all_sessions()` method to existing StreamManager
- [x] StreamManager already has per-session channel management via connections HashMap
- [x] Session isolation via internal HashMap<SessionId, HashMap<ConnectionId, Sender>>

### Phase 2: Server Integration (🔄 IN PROGRESS)
- [ ] Add `stream_manager: Arc<StreamManager<S>>` field to `HttpMcpServer`
- [ ] Update `SessionMcpHandler` to use shared StreamManager instance
- [ ] Remove complex `GlobalStreamManagerRegistry` approach
- [ ] Update constructors to pass shared StreamManager

### Phase 3: Testing (⏳ PENDING)
- [ ] Test session isolation with internal HashMap management
- [ ] Test server-wide broadcasts via `broadcast_to_all_sessions()`
- [ ] Test connection reuse with shared StreamManager instance  
- [ ] MCP compliance validation

## Migration Path

### Before (Broken)
```rust
// Each HTTP connection created new StreamManager
let handler = SessionMcpHandler::with_storage(config, dispatcher, storage, stream_config);
// Problem: New StreamManager per connection, no global coordination
```

### After (Fixed)
```rust  
// Single shared StreamManager at server level
let stream_manager = Arc::new(StreamManager::with_config(storage, stream_config));
let server = HttpMcpServer::new(config, dispatcher, stream_manager.clone());

// All handlers share the same StreamManager instance
let handler = SessionMcpHandler::new(config, dispatcher, stream_manager);
// Solution: Session isolation via internal HashMap + global coordination via shared instance
```

## Consequences

### Positive
- ✅ Fixes critical SSE notification delivery blocking issue
- ✅ Enables both session-specific and server-wide notifications
- ✅ Maintains MCP specification compliance
- ✅ Improves resource efficiency with single shared instance
- ✅ Provides clear, simple mental model for notification routing
- ✅ Eliminates complex registry pattern - much simpler architecture

### Negative  
- ⚠️ Requires adding broadcast_to_all_sessions() method to StreamManager
- ⚠️ Need to ensure proper cleanup of session channels on disconnect

### Neutral
- Single instance with internal management is standard architectural pattern
- Aligns perfectly with MCP specification requirements
- Much simpler to understand and maintain than registry approach

## Alternatives Considered

### Alternative 1: Pure Session Isolation
**Rejected**: Cannot support server-wide broadcasts needed for operational tooling

### Alternative 2: Pure Shared Architecture  
**Rejected**: Violates MCP session isolation requirements

### Alternative 3: Event Bus Pattern
**Considered**: Would work but adds unnecessary complexity for this specific problem

## Decision Rationale

The hybrid approach is the **only architecture** that satisfies both:
1. **MCP Specification**: Session isolation requirement
2. **Operational Requirements**: Server-wide broadcast capability

This architectural pattern (registry managing isolated instances with global coordination) is a well-established solution for similar problems in distributed systems.

---

**Author**: Claude Code  
**Reviewers**: N/A (Emergency architectural fix)  
**Related Issues**: Critical SSE notification delivery blocking (Day 3+)