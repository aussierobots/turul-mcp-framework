# TODO: Streamable HTTP Implementation Analysis & Session Management

## ULTRATHINK: Current Implementation vs MCP 2025-06-18 Streamable HTTP Specification

### Current State Analysis

**What We Have:**
- `http-mcp-server` crate with basic HTTP server
- Basic SSE support via `SseManager` and `SseStreamBody`
- Protocol version detection (V2024_11_05, V2025_03_26, V2025_06_18)
- Session header extraction (`Mcp-Session-Id`)
- JSON-RPC over HTTP POST
- Basic GET endpoint for SSE (limited implementation)

**Critical Gaps Found:**
1. **Session Management**: Current implementation lacks proper session lifecycle management
2. **SSE Stream Resumability**: No support for `Last-Event-ID` header or event replay
3. **Proper HTTP Status Codes**: Missing 202 Accepted for notifications/responses
4. **Session ID Generation**: No automatic session ID assignment during initialization
5. **DELETE Session Termination**: Missing explicit session termination endpoint
6. **Multiple Stream Support**: No per-stream event ID management
7. **Stream Response Integration**: SSE streams not properly integrated with JSON-RPC responses

### MCP 2025-06-18 Specification Requirements

#### Core Requirements
- [x] Single HTTP endpoint supporting POST and GET
- [x] POST for JSON-RPC messages with `Accept: application/json, text/event-stream`
- [x] GET for SSE streams with `Accept: text/event-stream`
- [ ] **202 Accepted** for notifications/responses (currently using 204 No Content)
- [ ] **Proper Content-Type switching** between `application/json` and `text/event-stream`
- [ ] **Session ID assignment** in `InitializeResult` response via `Mcp-Session-Id` header
- [ ] **Session validation** on subsequent requests
- [ ] **DELETE support** for explicit session termination

#### SSE Stream Requirements
- [x] Basic SSE event formatting
- [ ] **Event IDs** for resumability (`id` field in SSE events)
- [ ] **Last-Event-ID** header support for stream resumption
- [ ] **Per-stream event management** (not broadcast across streams)
- [ ] **Stream closure** after JSON-RPC response sent
- [ ] **Multiple concurrent streams** per client support

#### Security Requirements
- [x] Origin header validation (via CORS)
- [x] Localhost binding (configurable)
- [ ] **Proper authentication** (not implemented)

## Session Management System Design

### Trait-Based Architecture

```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    // Core session operations
    async fn create_session(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error>;
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error>;
    async fn update_session(&self, session_id: &str, info: SessionInfo) -> Result<(), Self::Error>;
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error>;
    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error>;
    
    // Stream management
    async fn create_stream(&self, session_id: &str, stream_id: String) -> Result<StreamInfo, Self::Error>;
    async fn get_stream(&self, session_id: &str, stream_id: &str) -> Result<Option<StreamInfo>, Self::Error>;
    async fn update_stream(&self, session_id: &str, stream_id: &str, info: StreamInfo) -> Result<(), Self::Error>;
    async fn delete_stream(&self, session_id: &str, stream_id: &str) -> Result<bool, Self::Error>;
    
    // Event management for resumability
    async fn store_event(&self, session_id: &str, stream_id: &str, event_id: u64, event: SseEvent) -> Result<(), Self::Error>;
    async fn get_events_after(&self, session_id: &str, stream_id: &str, after_event_id: u64) -> Result<Vec<(u64, SseEvent)>, Self::Error>;
    
    // Cleanup
    async fn expire_sessions(&self, older_than: std::time::SystemTime) -> Result<Vec<String>, Self::Error>;
    async fn cleanup(&self) -> Result<(), Self::Error>;
}
```

### Implementation Priorities

#### 1. InMemory SessionStorage (PRIORITY 1)
- [x] Basic session management exists in `SessionManager`
- [ ] **Refactor to trait-based architecture**
- [ ] **Add proper stream management**
- [ ] **Add event storage for resumability**
- [ ] **Add session expiration**

#### 2. SQLite SessionStorage (PRIORITY 2)
- [ ] **Design schema** for sessions, streams, and events
- [ ] **Implement persistence** with proper indexing
- [ ] **Add connection pooling**
- [ ] **Handle database migrations**

#### 3. AWS DynamoDB + SNS SessionStorage (PRIORITY 3)
- [ ] **Extend lambda-mcp-server work**
- [ ] **DynamoDB tables** for sessions/streams/events
- [ ] **SNS for cross-instance notifications**
- [ ] **Handle eventual consistency**

#### 4. NATS SessionStorage (PRIORITY 4)
- [ ] **JetStream for persistence**
- [ ] **Key-Value store for sessions**
- [ ] **Stream processing for events**
- [ ] **Distributed coordination**

## Key Files to Modify

### Core HTTP Server
- `crates/http-mcp-server/src/handler.rs` - Add proper status codes and session handling
- `crates/http-mcp-server/src/server.rs` - Add DELETE endpoint support
- `crates/http-mcp-server/src/protocol.rs` - Enhanced session ID management

### Session Management
- `crates/mcp-server/src/session.rs` - Refactor to trait-based architecture
- `crates/http-mcp-server/src/session_handler.rs` - Implement SessionStorage integration

### SSE Implementation
- `crates/http-mcp-server/src/sse.rs` - Add event IDs and resumability
- Add new `crates/http-mcp-server/src/stream_manager.rs` - Per-stream event management