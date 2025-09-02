# turul-mcp-session-storage

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-session-storage.svg)](https://crates.io/crates/turul-mcp-session-storage)
[![Documentation](https://docs.rs/turul-mcp-session-storage/badge.svg)](https://docs.rs/turul-mcp-session-storage)

Pluggable session storage backends for the turul-mcp-framework, supporting everything from in-memory development to distributed production deployments.

## Overview

`turul-mcp-session-storage` provides the `SessionStorage` trait and multiple implementations for persisting MCP session data, state, and SSE events across different storage backends.

## Features

- ✅ **Pluggable Architecture** - Swap backends without code changes
- ✅ **Production Ready** - Multiple production-grade backends  
- ✅ **Session Persistence** - Sessions survive server restarts
- ✅ **State Management** - Type-safe session state storage
- ✅ **SSE Event Storage** - Event replay for SSE resumability
- ✅ **Automatic Cleanup** - TTL-based session expiry
- ✅ **Multi-Instance Support** - Distributed session sharing

## Storage Backends

| Backend | Use Case | Features | Production Ready |
|---------|----------|----------|------------------|
| **InMemory** | Development/Testing | Fast, simple | ✅ Dev only |
| **SQLite** | Single-instance production | File-based, ACID | ✅ Yes |
| **PostgreSQL** | Multi-instance production | Distributed, scalable | ✅ Yes |
| **DynamoDB** | Serverless/AWS Lambda | Auto-scaling, managed | ✅ Yes |

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-session-storage = { version = "0.1", features = ["sqlite"] }
turul-mcp-server = "0.1"
```

### In-Memory (Development)

```rust
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::InMemorySessionStorage;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In-memory storage (default)
    let storage = Arc::new(InMemorySessionStorage::new());
    
    let server = McpServer::builder()
        .with_session_storage(storage)
        .tool(/* your tools */)
        .build()?
        .start()
        .await?;
        
    Ok(())
}
```

### SQLite (Single Instance)

```rust
use turul_mcp_session_storage::SqliteSessionStorage;
use std::sync::Arc;

// SQLite with file persistence
let storage = Arc::new(
    SqliteSessionStorage::new("sessions.db").await?
);

let server = McpServer::builder()
    .with_session_storage(storage)
    .build()?;
```

### PostgreSQL (Multi-Instance)

```rust
use turul_mcp_session_storage::PostgreSqlSessionStorage;
use std::sync::Arc;

// PostgreSQL for distributed deployments
let storage = Arc::new(
    PostgreSqlSessionStorage::new("postgresql://user:pass@localhost/mcpdb").await?
);

let server = McpServer::builder()
    .with_session_storage(storage)
    .build()?;
```

### DynamoDB (Serverless)

```rust
use turul_mcp_session_storage::DynamoDbSessionStorage;
use std::sync::Arc;

// DynamoDB for AWS Lambda deployments
let storage = Arc::new(
    DynamoDbSessionStorage::new().await?  // Auto table creation
);

let server = McpServer::builder()
    .with_session_storage(storage)
    .build()?;
```

## Session Management

### Session Lifecycle

Sessions follow this lifecycle:

1. **Creation** - Server assigns UUID v7 session ID
2. **Usage** - Tools read/write session state
3. **Persistence** - State automatically saved to storage
4. **Expiry** - TTL-based cleanup (default 30 minutes)
5. **Cleanup** - Automatic background cleanup

### Session State API

```rust
use turul_mcp_server::SessionContext;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UserPreferences {
    theme: String,
    language: String,
    notifications: bool,
}

// In your tool implementation
async fn handle_user_preferences(session: SessionContext) -> Result<(), Box<dyn std::error::Error>> {
    // Get typed state
    let prefs: Option<UserPreferences> = session.get_typed_state("user_prefs").await?;
    
    let mut preferences = prefs.unwrap_or(UserPreferences {
        theme: "light".to_string(),
        language: "en".to_string(),
        notifications: true,
    });
    
    // Modify preferences
    preferences.theme = "dark".to_string();
    
    // Save typed state
    session.set_typed_state("user_prefs", &preferences).await?;
    
    // Remove state when no longer needed
    session.remove_state("user_prefs").await?;
    
    Ok(())
}
```

### Session Information

```rust
async fn session_info(session: SessionContext) -> Result<(), Box<dyn std::error::Error>> {
    // Session metadata
    println!("Session ID: {}", session.session_id());
    println!("Created: {:?}", session.created_at());
    println!("Last accessed: {:?}", session.last_accessed());
    
    // List all state keys
    let keys = session.list_state_keys().await?;
    println!("State keys: {:?}", keys);
    
    // Get raw state as JSON
    let raw_state = session.get_state("some_key").await?;
    if let Some(value) = raw_state {
        println!("Raw value: {}", value);
    }
    
    Ok(())
}
```

## SSE Event Storage

### Event Persistence

All session backends support SSE event storage for resumability:

```rust
use turul_mcp_server::SessionContext;

async fn send_progress_with_persistence(session: SessionContext) -> Result<(), Box<dyn std::error::Error>> {
    // Progress notifications are automatically stored
    session.notify_progress(
        "long-task", 
        50.0, 
        Some(100.0), 
        Some("Processing files...".to_string())
    ).await?;
    
    // Client can reconnect and replay from last-event-id
    Ok(())
}
```

### Event Replay

SSE clients can resume from any point using `Last-Event-ID`:

```
GET /mcp HTTP/1.1
Accept: text/event-stream
Last-Event-ID: event-123
Mcp-Session-Id: sess-456
```

The storage backend will replay all events after `event-123`.

## Backend Configuration

### SQLite Configuration

```rust
use turul_mcp_session_storage::{SqliteSessionStorage, SqliteConfig};

let config = SqliteConfig {
    database_path: "sessions.db".to_string(),
    session_ttl_seconds: 3600,  // 1 hour
    cleanup_interval_seconds: 300,  // 5 minutes
    max_events_per_session: 1000,
};

let storage = SqliteSessionStorage::with_config(config).await?;
```

### PostgreSQL Configuration

```rust
use turul_mcp_session_storage::{PostgreSqlSessionStorage, PostgreSqlConfig};

let config = PostgreSqlConfig {
    connection_string: "postgresql://user:pass@localhost/mcpdb".to_string(),
    table_prefix: "mcp_".to_string(),
    session_ttl_seconds: 1800,  // 30 minutes
    max_pool_size: 10,
    cleanup_interval_seconds: 600,  // 10 minutes
};

let storage = PostgreSqlSessionStorage::with_config(config).await?;
```

### DynamoDB Configuration

```rust
use turul_mcp_session_storage::{DynamoDbSessionStorage, DynamoDbConfig};

let config = DynamoDbConfig {
    table_name: "mcp-sessions".to_string(),
    events_table_name: "mcp-session-events".to_string(),
    region: "us-east-1".to_string(),
    session_ttl_seconds: 1800,
    auto_create_tables: true,
};

let storage = DynamoDbSessionStorage::with_config(config).await?;
```

## Production Deployment

### Single-Instance with SQLite

Perfect for single-server deployments:

```rust
use turul_mcp_session_storage::SqliteSessionStorage;

// Production SQLite setup
let storage = SqliteSessionStorage::new("/var/lib/mcp/sessions.db").await?;

// Configure for production
let storage = SqliteSessionStorage::with_config(SqliteConfig {
    database_path: "/var/lib/mcp/sessions.db".to_string(),
    session_ttl_seconds: 7200,  // 2 hours
    cleanup_interval_seconds: 600,  // 10 minutes cleanup
    max_events_per_session: 5000,  // Large event buffer
}).await?;
```

### Multi-Instance with PostgreSQL

For load-balanced deployments:

```rust
use turul_mcp_session_storage::PostgreSqlSessionStorage;

// Production PostgreSQL setup
let database_url = std::env::var("DATABASE_URL")?;
let storage = PostgreSqlSessionStorage::new(&database_url).await?;

// All server instances share the same sessions
let server = McpServer::builder()
    .bind("0.0.0.0:3000")
    .with_session_storage(Arc::new(storage))
    .build()?;
```

### Serverless with DynamoDB

For AWS Lambda and serverless:

```rust
use turul_mcp_session_storage::DynamoDbSessionStorage;

// Serverless DynamoDB setup
let storage = DynamoDbSessionStorage::new().await?;  // Uses AWS SDK defaults

// Perfect for Lambda deployments
let lambda_server = turul_mcp_aws_lambda::LambdaMcpServerBuilder::new()
    .storage(Arc::new(storage))
    .build()
    .await?;
```

## Custom Storage Backend

### Implementing SessionStorage

```rust
use turul_mcp_session_storage::{SessionStorage, SessionData, SessionEvent};
use async_trait::async_trait;
use uuid::Uuid;
use std::collections::HashMap;

pub struct RedisSessionStorage {
    client: redis::Client,
}

#[async_trait]
impl SessionStorage for RedisSessionStorage {
    type Error = redis::RedisError;
    
    async fn create_session(&self, session_id: Uuid) -> Result<(), Self::Error> {
        let mut conn = self.client.get_async_connection().await?;
        let session_data = SessionData::new(session_id);
        let serialized = serde_json::to_string(&session_data)?;
        
        redis::cmd("SETEX")
            .arg(format!("session:{}", session_id))
            .arg(1800) // 30 minute TTL
            .arg(serialized)
            .query_async(&mut conn)
            .await
    }
    
    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionData>, Self::Error> {
        let mut conn = self.client.get_async_connection().await?;
        let result: Option<String> = redis::cmd("GET")
            .arg(format!("session:{}", session_id))
            .query_async(&mut conn)
            .await?;
            
        match result {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }
    
    async fn update_session(&self, session_data: &SessionData) -> Result<(), Self::Error> {
        // Implementation for updating session
        todo!()
    }
    
    async fn delete_session(&self, session_id: Uuid) -> Result<(), Self::Error> {
        // Implementation for deleting session
        todo!()
    }
    
    // ... implement remaining methods
}
```

## Error Handling

### Storage Errors

Each backend defines its own error type:

```rust
use turul_mcp_session_storage::{SqliteSessionStorage, SqliteError};

match storage.get_session(session_id).await {
    Ok(Some(session)) => {
        // Handle session
    }
    Ok(None) => {
        // Session not found
    }
    Err(SqliteError::Database(e)) => {
        // Database connection error
    }
    Err(SqliteError::Serialization(e)) => {
        // JSON serialization error
    }
}
```

### Graceful Degradation

The framework provides graceful degradation when storage fails:

```rust
// Session operations that fail gracefully
if let Err(e) = session.set_typed_state("key", &value).await {
    tracing::warn!("Failed to persist session state: {}", e);
    // Operation continues without state persistence
}
```

## Performance & Monitoring

### Connection Pooling

Production backends use connection pooling:

```rust
// PostgreSQL with custom pool size
let storage = PostgreSqlSessionStorage::with_config(PostgreSqlConfig {
    max_pool_size: 20,  // Increase for high concurrency
    ..Default::default()
}).await?;
```

### Metrics Collection

```rust
// Session metrics (example - implement in your monitoring)
async fn collect_session_metrics(storage: &dyn SessionStorage) {
    let active_sessions = storage.count_sessions().await?;
    let cleanup_stats = storage.cleanup_expired_sessions().await?;
    
    // Send to metrics system
    metrics::gauge!("mcp.sessions.active", active_sessions as f64);
    metrics::counter!("mcp.sessions.cleaned_up", cleanup_stats.removed as u64);
}
```

## Testing

### Test Utilities

```rust
use turul_mcp_session_storage::test_utils::*;

#[tokio::test]
async fn test_session_storage() {
    let storage = InMemorySessionStorage::new();
    
    // Test session lifecycle
    test_session_lifecycle(&storage).await;
    
    // Test state management
    test_state_operations(&storage).await;
    
    // Test event storage
    test_event_operations(&storage).await;
}
```

### Integration Tests

```bash
# Test all backends
cargo test --package turul-mcp-session-storage --all-features

# Test specific backend
cargo test --package turul-mcp-session-storage --features sqlite

# Test with real databases (requires setup)
cargo test --package turul-mcp-session-storage --features postgres -- --ignored
```

## Feature Flags

```toml
[dependencies]
turul-mcp-session-storage = { version = "0.1", features = ["sqlite", "postgres"] }
```

- `default` - Only InMemory backend
- `sqlite` - SQLite backend  
- `postgres` - PostgreSQL backend
- `dynamodb` - DynamoDB backend
- `redis` - Redis backend (planned)

## Migration Guide

### Upgrading Storage Backends

When upgrading from InMemory to persistent storage:

```rust
// Before (development)
let storage = Arc::new(InMemorySessionStorage::new());

// After (production)
let storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);

// Server code stays the same!
let server = McpServer::builder()
    .with_session_storage(storage)  // No changes needed
    .build()?;
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.