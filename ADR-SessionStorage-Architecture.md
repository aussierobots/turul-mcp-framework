# ADR: Session Storage Architecture

**Status**: Implemented  
**Date**: 2025-08-30  
**Decision**: Unified session storage architecture across MCP framework components

## Context

The MCP framework requires session management for:
- Tool execution state persistence across requests
- SSE event storage and resumability 
- Multi-instance deployment support
- Different persistence backends (InMemory, PostgreSQL, SQLite)

The framework consists of multiple crates that need to work together for session management:
- `mcp-session-storage`: Storage abstraction and implementations
- `mcp-server`: Core MCP server with tool execution
- `mcp-json-rpc-server`: JSON-RPC handling and session contexts
- `http-mcp-server`: HTTP transport with SSE streaming

## Decision

### Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   mcp-server    │    │ mcp-json-rpc-    │    │  http-mcp-server    │
│                 │    │     server       │    │                     │
│  ┌─────────────┐│    │ ┌──────────────┐ │    │ ┌─────────────────┐ │
│  │SessionManager││◄──►││SessionContext│ │    │ │SessionMcpHandler│ │
│  │             ││    │ │              │ │    │ │                 │ │
│  └─────────────┘│    │ └──────────────┘ │    │ └─────────────────┘ │
│         │       │    └──────────────────┘    │          │          │
└─────────┼───────┘                            └──────────┼──────────┘
          │                                               │
          │              ┌─────────────────────────────────────┐
          └──────────────►│       mcp-session-storage           │
                         │                                     │
                         │ ┌─────────────┐ ┌─────────────────┐ │
                         │ │SessionStorage│ │ Implementations │ │
                         │ │    Trait    │ │                 │ │
                         │ │             │ │ • InMemory      │ │
                         │ └─────────────┘ │ • PostgreSQL    │ │
                         │                 │ • SQLite        │ │
                         │                 └─────────────────┘ │
                         └─────────────────────────────────────┘
```

### Component Responsibilities

#### 1. `mcp-session-storage` Crate
- **Purpose**: Provides SessionStorage trait abstraction and backend implementations
- **Responsibilities**:
  - Define SessionStorage trait interface
  - Implement InMemory, PostgreSQL, SQLite backends
  - Handle session state persistence (key-value pairs)
  - Manage SSE event storage and resumability
  - Provide session expiration and cleanup

#### 2. `mcp-server` Crate  
- **Purpose**: Core MCP server with tool execution and session management
- **Responsibilities**:
  - Own the SessionManager that orchestrates session lifecycle
  - Accept SessionStorage backend via builder pattern
  - Create SessionContext for tool execution
  - Manage session creation, initialization, and cleanup

#### 3. `mcp-json-rpc-server` Crate
- **Purpose**: JSON-RPC protocol handling and session context provision
- **Responsibilities**:
  - Provide SessionContext to tool handlers
  - Bridge between HTTP requests and session state
  - Handle session ID extraction and validation

#### 4. `http-mcp-server` Crate
- **Purpose**: HTTP transport layer with SSE streaming
- **Responsibilities**:
  - Handle HTTP requests and SSE connections
  - Manage session headers (Mcp-Session-Id)
  - Use SessionStorage for SSE event persistence
  - Coordinate with mcp-server's SessionManager

### Data Flow

1. **Session Creation**: 
   - Client sends `initialize` request
   - `mcp-server` creates session via SessionStorage
   - Returns `Mcp-Session-Id` header

2. **Tool Execution**:
   - Client sends tool request with session ID
   - `mcp-json-rpc-server` creates SessionContext
   - Tool uses `SessionContext.set_state()` to persist data
   - State is stored via SessionStorage backend

3. **SSE Events**:
   - Tools send notifications via SessionContext
   - Events stored in SessionStorage for resumability
   - `http-mcp-server` streams events to clients

### SessionStorage Interface

```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    // Session Management
    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo>;
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>>;
    async fn set_session_state(&self, session_id: &str, key: &str, value: Value) -> Result<()>;
    async fn get_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>>;
    
    // SSE Event Management  
    async fn store_event(&self, session_id: &str, event: SseEvent) -> Result<SseEvent>;
    async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> Result<Vec<SseEvent>>;
    
    // Cleanup and Maintenance
    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>>;
}
```

### Backend Implementations

#### InMemory Storage (Default)
- **Use Case**: Development, testing, single-instance deployment
- **Characteristics**: Fast, no persistence, data lost on restart

#### PostgreSQL Storage
- **Use Case**: Multi-instance production deployment
- **Characteristics**: Persistent, shared across instances, ACID transactions

#### SQLite Storage  
- **Use Case**: Single-instance production with persistence
- **Characteristics**: File-based, persistent, local storage

#### AWS DynamoDB Storage
- **Use Case**: Serverless/Lambda deployment, AWS-native applications
- **Characteristics**: Fully managed, automatic scaling, integrated with AWS services
- **Features**: Global tables for multi-region, TTL for automatic cleanup, streams for notifications

## Configuration Patterns

### Default (InMemory)
```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(my_tool)
    .build()?; // Uses InMemory storage automatically
```

### PostgreSQL Backend
```rust
let postgres_storage = Arc::new(PostgresSessionStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server") 
    .tool(my_tool)
    .with_session_storage(postgres_storage)
    .build()?;
```

### SQLite Backend
```rust
let sqlite_storage = Arc::new(SqliteSessionStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server")
    .tool(my_tool) 
    .with_session_storage(sqlite_storage)
    .build()?;
```

### AWS DynamoDB Backend
```rust
let dynamodb_config = DynamoDbConfig {
    table_name: "mcp-sessions".to_string(),
    region: "us-east-1".to_string(),
    session_ttl_hours: 24,
    ..Default::default()
};
let dynamodb_storage = Arc::new(DynamoDbSessionStorage::with_config(dynamodb_config).await?);
let server = McpServer::builder()
    .name("my-server")
    .tool(my_tool)
    .with_session_storage(dynamodb_storage)
    .build()?;
```

## Benefits

1. **Pluggable Backends**: Easy to switch between storage implementations
2. **Clean Separation**: Each crate has focused responsibilities  
3. **Scalability**: PostgreSQL backend enables multi-instance deployment
4. **Developer Experience**: Zero-config default (InMemory) with easy customization
5. **Persistence**: Session state survives server restarts with persistent backends
6. **Resumability**: SSE events stored for client reconnection

## Trade-offs

### Accepted
- **Async Throughout**: All SessionStorage operations are async for consistency
- **Trait Objects**: Use `Arc<dyn SessionStorage>` for runtime polymorphism
- **Memory Usage**: InMemory backend uses more memory but provides better performance

### Rejected
- **Separate Storage Types**: Could have different traits for session vs SSE storage, but unified interface is simpler
- **Sync Interface**: Sync interface would limit backend options (no database backends)

## Implementation Notes

1. **UUID v7**: All session IDs use UUID v7 for temporal ordering
2. **Error Handling**: SessionStorage errors are propagated as framework errors
3. **Cleanup**: Automatic session expiration every 60 seconds (configurable)
4. **Testing**: Each backend provides test implementations for consistent behavior

## Migration Path

- **Phase 1**: Implement SessionStorage trait in all backends ✅
- **Phase 2**: Integrate with mcp-server SessionManager ✅  
- **Phase 3**: Update examples to demonstrate different backends 🔄
- **Phase 4**: Add monitoring and metrics for session storage