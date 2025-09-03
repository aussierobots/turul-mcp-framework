# turul-http-mcp-server

[![Crates.io](https://img.shields.io/crates/v/turul-http-mcp-server.svg)](https://crates.io/crates/turul-http-mcp-server)
[![Documentation](https://docs.rs/turul-http-mcp-server/badge.svg)](https://docs.rs/turul-http-mcp-server)

HTTP transport layer for Model Context Protocol (MCP) servers with full MCP 2025-06-18 Streamable HTTP compliance and SSE support.

## Overview

`turul-http-mcp-server` provides the HTTP transport layer for the turul-mcp-framework, implementing MCP Streamable HTTP with Server-Sent Events (SSE) for real-time notifications and session resumability.

## Features

- ✅ **MCP 2025-06-18 Streamable HTTP** - Full protocol compliance with SSE streaming
- ✅ **Session Management** - UUID v7 session IDs with automatic cleanup
- ✅ **SSE Resumability** - Last-Event-ID support with event replay
- ✅ **CORS Support** - Browser client compatibility with configurable origins
- ✅ **Protocol Version Detection** - Automatic feature flags based on client capabilities
- ✅ **Pluggable Storage** - InMemory, SQLite, PostgreSQL, DynamoDB session backends
- ✅ **Performance Optimized** - Connection pooling, streaming responses, efficient JSON-RPC dispatch

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-http-mcp-server = "0.1.1"
turul-mcp-server = "0.1.1"
turul-mcp-derive = "0.1.1"
```

### Basic HTTP MCP Server

```rust
use turul_http_mcp_server::{HttpMcpServer, ServerConfig};
use turul_mcp_server::McpServer;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::McpResult;
use std::net::SocketAddr;

#[mcp_tool(name = "echo", description = "Echo back the provided message")]
async fn echo_tool(
    #[param(description = "Message to echo back")] message: String,
) -> McpResult<String> {
    Ok(format!("Echo: {}", message))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create MCP server with tools
    let mcp_server = McpServer::builder()
        .name("Echo Server")
        .version("1.0.0")
        .tool_fn(echo_tool)
        .build()?;

    // Configure HTTP server
    let config = ServerConfig {
        bind_address: "127.0.0.1:3000".parse()?,
        mcp_path: "/mcp".to_string(),
        enable_cors: true,
        enable_sse: true,
        ..Default::default()
    };

    // Start HTTP server
    let server = HttpMcpServer::new(config, mcp_server).await?;
    println!("MCP server running on http://127.0.0.1:3000/mcp");
    
    server.serve().await?;
    Ok(())
}
```

## Streamable HTTP Architecture

### Protocol Flow

The HTTP MCP server implements the MCP 2025-06-18 Streamable HTTP specification:

```
┌─────────────────────────────────────────────────┐
│                MCP Client                       │
├─────────────────────────────────────────────────┤
│  POST /mcp + Accept: application/json           │  ← JSON-RPC requests
│  GET  /mcp + Accept: text/event-stream          │  ← SSE notifications
├─────────────────────────────────────────────────┤
│            turul-http-mcp-server                │
│  ├─ SessionMcpHandler                          │  ← Session management
│  ├─ StreamManager                              │  ← SSE event streaming  
│  ├─ NotificationBroadcaster                    │  ← Real-time notifications
│  └─ JsonRpcDispatcher                          │  ← JSON-RPC routing
├─────────────────────────────────────────────────┤
│            turul-mcp-server                     │  ← Core framework
└─────────────────────────────────────────────────┘
```

### Session Management

Sessions are managed with UUID v7 for temporal ordering:

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, SessionMcpHandler};
use turul_mcp_session_storage::SqliteSessionStorage;
use std::sync::Arc;

let storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);

let server = HttpMcpServerBuilder::new()
    .bind_address("0.0.0.0:3000".parse()?)
    .session_storage(storage)  // Persistent session storage
    .enable_sse(true)
    .build()
    .await?;
```

### SSE Event Streaming

Real-time notifications are delivered via Server-Sent Events:

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, SessionContext};

#[mcp_tool(name = "long_task", description = "Long-running task with progress")]
async fn long_task(
    #[param(description = "Task duration in seconds")] duration: u32,
    session: SessionContext,  // Automatic session injection
) -> McpResult<String> {
    for i in 1..=duration {
        // Send progress notification via SSE
        session.notify_progress(
            "long-task", 
            i as f64, 
            Some(duration as f64), 
            Some(format!("Step {} of {}", i, duration))
        ).await?;
        
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    
    Ok("Task completed".to_string())
}
```

## Advanced Configuration

### Custom Server Builder

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, ServerConfig, CorsLayer};
use turul_mcp_session_storage::PostgreSqlSessionStorage;

let cors = CorsLayer::new()
    .allow_origins(vec!["https://myapp.com".to_string()])
    .allow_credentials(true);

let storage = Arc::new(
    PostgreSqlSessionStorage::new("postgresql://user:pass@localhost/db").await?
);

let server = HttpMcpServerBuilder::new()
    .bind_address("0.0.0.0:8080".parse()?)
    .mcp_path("/api/mcp")
    .session_storage(storage)
    .cors(cors)
    .max_body_size(2 * 1024 * 1024)  // 2MB
    .enable_sse(true)
    .build()
    .await?;
```

### Protocol Version Detection

The server automatically detects client capabilities:

```rust
// Client sends MCP-Protocol-Version header
// Server responds with appropriate feature set:

// V2024_11_05: Basic MCP without streamable HTTP
// V2025_03_26: Streamable HTTP support
// V2025_06_18: Full feature set with _meta, cursor, progressToken
```

## CORS Configuration

### Browser Client Support

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, CorsLayer};

// Allow all origins (development)
let server = HttpMcpServerBuilder::new()
    .enable_cors(true)  // Uses permissive defaults
    .build()
    .await?;

// Custom CORS configuration (production)
let cors = CorsLayer::new()
    .allow_origins(vec![
        "https://app.example.com".to_string(),
        "https://admin.example.com".to_string(),
    ])
    .allow_methods(vec!["GET", "POST", "OPTIONS"])
    .allow_headers(vec!["Content-Type", "Accept", "MCP-Protocol-Version"])
    .allow_credentials(true);

let server = HttpMcpServerBuilder::new()
    .cors(cors)
    .build()
    .await?;
```

## Session Storage Backends

### InMemory (Development)

```rust
use turul_http_mcp_server::HttpMcpServerBuilder;
use turul_mcp_session_storage::InMemorySessionStorage;

let server = HttpMcpServerBuilder::new()
    .session_storage(Arc::new(InMemorySessionStorage::new()))  // Default
    .build()
    .await?;
```

### SQLite (Single Instance)

```rust
use turul_mcp_session_storage::SqliteSessionStorage;

let storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);
let server = HttpMcpServerBuilder::new()
    .session_storage(storage)
    .build()
    .await?;
```

### PostgreSQL (Multi-Instance)

```rust
use turul_mcp_session_storage::PostgreSqlSessionStorage;

let storage = Arc::new(
    PostgreSqlSessionStorage::new("postgresql://user:pass@localhost/mcp").await?
);
let server = HttpMcpServerBuilder::new()
    .session_storage(storage)
    .build()
    .await?;
```

### DynamoDB (Serverless)

```rust
use turul_mcp_session_storage::DynamoDbSessionStorage;

let storage = Arc::new(DynamoDbSessionStorage::new().await?);
let server = HttpMcpServerBuilder::new()
    .session_storage(storage)
    .build()
    .await?;
```

## Real-time Notifications

### SSE Event Types

The server supports all MCP notification types via SSE:

```rust
// Progress notifications
session.notify_progress("task-id", 50.0, Some(100.0), Some("Processing...")).await?;

// Resource update notifications  
session.notify_resource_updated("file:///config.json").await?;

// Prompt update notifications
session.notify_prompt_updated("greeting").await?;

// Log notifications
session.notify_log("info", serde_json::json!({
    "message": "Operation completed",
    "duration_ms": 1250
}), Some("component")).await?;
```

### SSE Resumability

Clients can resume from any event using Last-Event-ID:

```bash
# Initial connection
curl -N -H "Accept: text/event-stream" \
  -H "MCP-Session-Id: sess-123" \
  http://localhost:3000/mcp

# Resume from specific event
curl -N -H "Accept: text/event-stream" \
  -H "Last-Event-ID: event-456" \
  -H "MCP-Session-Id: sess-123" \
  http://localhost:3000/mcp
```

## Testing

### MCP Inspector Integration

Test your server with MCP Inspector:

```bash
# Start server
cargo run --example echo-server

# Connect MCP Inspector
# URL: http://localhost:3000/mcp
# Enable SSE notifications for real-time updates
```

### Manual Testing

```bash
# 1. Initialize session
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  -v  # Note the Mcp-Session-Id header in response

# 2. Call tool with session
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: <session-id>" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello MCP!"}}}'

# 3. Open SSE stream for notifications
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://localhost:3000/mcp
```

## Performance

### Optimized Architecture

- **Connection Pooling**: Database connections shared across requests
- **Streaming Responses**: Efficient SSE event delivery without buffering
- **JSON-RPC Dispatch**: Fast method routing with compile-time registration
- **Session Cleanup**: Automatic cleanup every 60 seconds with 30-minute expiry

### Monitoring

```rust
use turul_http_mcp_server::{ServerStats, StreamStats};

// Server statistics
let stats = server.stats().await;
println!("Active sessions: {}", stats.active_sessions);
println!("Total requests: {}", stats.total_requests);

// Stream statistics  
let stream_stats = server.stream_stats().await;
println!("Active streams: {}", stream_stats.active_streams);
println!("Events sent: {}", stream_stats.events_sent);
```

## Error Handling

### HTTP Error Responses

The server provides proper HTTP error responses with MCP-compliant JSON-RPC errors:

```rust
use turul_http_mcp_server::{HttpMcpError, Result};

// Tool implementation with error handling
#[mcp_tool(name = "validate", description = "Validate input")]
async fn validate_input(
    #[param(description = "Value to validate")] value: String,
) -> McpResult<String> {
    if value.is_empty() {
        return Err("Value cannot be empty".into());
    }
    
    if value.len() > 100 {
        return Err("Value too long (max 100 chars)".into());
    }
    
    Ok(format!("Valid: {}", value))
}
```

### Graceful Degradation

```rust
// Session operations fail gracefully
if let Err(e) = session.set_typed_state("key", &value).await {
    tracing::warn!("Failed to persist session state: {}", e);
    // Operation continues without state persistence
}
```

## Examples

### Complete Production Server

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, CorsLayer};
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::PostgreSqlSessionStorage;
use turul_mcp_derive::mcp_tool;
use std::sync::Arc;

#[mcp_tool(name = "status", description = "Check server status")]
async fn status_check() -> McpResult<serde_json::Value> {
    Ok(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": "1.0.0"
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Database connection for session storage
    let database_url = std::env::var("DATABASE_URL")?;
    let storage = Arc::new(PostgreSqlSessionStorage::new(&database_url).await?);

    // CORS configuration for production
    let cors = CorsLayer::new()
        .allow_origins(vec!["https://myapp.com".to_string()])
        .allow_credentials(true);

    // Build MCP server
    let mcp_server = McpServer::builder()
        .name("Production MCP Server")
        .version("1.0.0")
        .tool_fn(status_check)
        .build()?;

    // Build HTTP server
    let http_server = HttpMcpServerBuilder::new()
        .bind_address("0.0.0.0:8080".parse()?)
        .session_storage(storage)
        .cors(cors)
        .max_body_size(5 * 1024 * 1024)  // 5MB
        .enable_sse(true)
        .mcp_server(mcp_server)
        .build()
        .await?;

    println!("Production MCP server starting on http://0.0.0.0:8080/mcp");
    
    http_server.serve().await?;
    Ok(())
}
```

## Feature Flags

```toml
[dependencies]
turul-http-mcp-server = { version = "0.1.1", features = ["sse"] }
```

- `default` = `["sse"]` - Includes SSE support by default
- `sse` - Server-Sent Events streaming for real-time notifications

## Compatibility

### MCP Protocol Versions

- **2024-11-05**: Basic MCP without streamable HTTP
- **2025-03-26**: Streamable HTTP with SSE support
- **2025-06-18**: Full feature set with meta fields and enhanced capabilities

### HTTP Clients

- **MCP Inspector**: Full compatibility with browser-based testing
- **curl**: Command-line testing and integration
- **JavaScript/TypeScript**: Browser and Node.js client support
- **Python**: Using `requests` library with SSE support

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.