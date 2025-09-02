# turul-mcp-server

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-server.svg)](https://crates.io/crates/turul-mcp-server)
[![Documentation](https://docs.rs/turul-mcp-server/badge.svg)](https://docs.rs/turul-mcp-server)

High-level framework for building Model Context Protocol (MCP) servers with full MCP 2025-06-18 compliance.

## Overview

`turul-mcp-server` is the core framework crate that provides everything you need to build production-ready MCP servers. It offers four different approaches for tool creation, automatic session management, real-time SSE notifications, and pluggable storage backends.

## Features

- ✅ **MCP 2025-06-18 Compliance** - Full protocol compliance with latest features
- ✅ **Zero-Configuration** - Framework auto-determines ALL methods from types  
- ✅ **Four Tool Creation Levels** - Function/derive/builder/manual approaches
- ✅ **Real-time Notifications** - SSE streaming with JSON-RPC format
- ✅ **Session Management** - UUID v7 sessions with automatic cleanup
- ✅ **Pluggable Storage** - InMemory, SQLite, PostgreSQL, DynamoDB backends
- ✅ **Type Safety** - Compile-time schema generation and validation

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-server = "0.1"
turul-mcp-derive = "0.1"  # For derive macros
```

### Level 1: Function Macros (Ultra-Simple)

```rust
use turul_mcp_server::{McpServer, McpResult};
use turul_mcp_derive::mcp_tool;

#[mcp_tool(name = "calculator", description = "Add two numbers")]
async fn calculator(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .tool_fn(calculator)  // Just pass the function name!
        .build()?
        .start()
        .await?;
    Ok(())
}
```

### Level 2: Derive Macros (Struct-Based)

```rust
use turul_mcp_server::{McpServer, McpResult, SessionContext};
use turul_mcp_derive::McpTool;

#[derive(McpTool, Clone, Default)]
#[tool(name = "calculator", description = "Add two numbers")]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .tool(Calculator::default())
        .build()?
        .start()
        .await?;
    Ok(())
}
```

### Level 3: Runtime Builders

```rust
use turul_mcp_server::McpServer;
use turul_mcp_builders::ToolBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calculator = ToolBuilder::new("calculator")
        .description("Add two numbers")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .execute(|args| async move {
            let a = args["a"].as_f64().unwrap();
            let b = args["b"].as_f64().unwrap();
            Ok(json!({"result": a + b}))
        })
        .build()?;

    let server = McpServer::builder()
        .tool(calculator)
        .build()?
        .start()
        .await?;
    Ok(())
}
```

## Session Management & Storage

### Pluggable Storage Backends

```rust
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::{SqliteSessionStorage, PostgreSqlSessionStorage};
use std::sync::Arc;

// SQLite for single-instance deployments
let storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);

// PostgreSQL for multi-instance deployments  
let storage = Arc::new(PostgreSqlSessionStorage::new("postgresql://...").await?);

let server = McpServer::builder()
    .with_session_storage(storage)
    .tool(/* your tools */)
    .build()?;
```

### Session Context in Tools

```rust
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_derive::McpTool;

#[derive(McpTool, Clone, Default)]
#[tool(name = "stateful_counter", description = "Increment session counter")]
struct StatefulCounter;

impl StatefulCounter {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<i32> {
        if let Some(session) = session {
            // Get current counter or start at 0
            let current: i32 = session.get_typed_state("counter").await?.unwrap_or(0);
            let new_count = current + 1;
            
            // Save updated counter
            session.set_typed_state("counter", &new_count).await?;
            
            // Send progress notification
            session.notify_progress("counting", new_count as f64, None, 
                Some(format!("Count: {}", new_count))).await?;
            
            Ok(new_count)
        } else {
            Ok(0) // No session available
        }
    }
}
```

## Real-time Notifications

### SSE Streaming

The framework automatically provides SSE endpoints for real-time notifications:

- **POST** `/mcp` + `Accept: application/json` → JSON responses
- **GET** `/mcp` + `Accept: text/event-stream` → SSE stream
- **Session isolation** → Each session gets independent notification channels

```rust
// Notifications are sent automatically from tools
session.notify_progress("task-123", 75.0, Some(100.0), 
    Some("Processing files...".to_string())).await?;

session.notify_log("info", "Operation completed successfully").await?;
```

## Server Configuration

### HTTP Server with Custom Port

```rust
use turul_mcp_server::McpServer;

let server = McpServer::builder()
    .bind("127.0.0.1:8080")  // Custom bind address
    .tool(/* your tools */)
    .build()?
    .start()
    .await?;
```

### Production Configuration

```rust
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::PostgreSqlSessionStorage;
use std::sync::Arc;

let storage = Arc::new(PostgreSqlSessionStorage::new(
    std::env::var("DATABASE_URL")?
).await?);

let server = McpServer::builder()
    .name("production-server")
    .version("1.0.0") 
    .bind("0.0.0.0:3000")
    .with_session_storage(storage)
    .tool(/* your tools */)
    .build()?
    .start()
    .await?;
```

## Protocol Compliance

### MCP 2025-06-18 Features

- ✅ **Initialize/Initialized** - Capability negotiation
- ✅ **Tools** - Dynamic tool calling with schema validation
- ✅ **Resources** - File and data resource access
- ✅ **Prompts** - Template-based prompt management
- ✅ **Sampling** - AI model sampling integration
- ✅ **Completion** - Context-aware autocompletion
- ✅ **Roots** - Secure file system access
- ✅ **Notifications** - Real-time event streaming
- ✅ **Cancellation** - Request cancellation with progress tokens
- ✅ **Logging** - Structured logging with multiple levels

### Zero-Configuration Promise

Users never specify method strings - the framework automatically determines all MCP methods from types:

```rust
let server = McpServer::builder()
    .tool(calculator)                        // → tools/call
    .notification_type::<ProgressNotification>() // → notifications/progress
    .resource(file_resource)                 // → resources/read
    .build()?;
```

## Examples

The [turul-mcp-framework repository](https://github.com/anthropics/turul-mcp-framework) contains 25+ comprehensive examples:

- **minimal-server** - Simplest possible server
- **stateful-server** - Session management patterns
- **notification-server** - Real-time SSE notifications
- **comprehensive-server** - All MCP features demonstrated
- **lambda-mcp-server** - AWS Lambda deployment

## Architecture

### Framework Layers

```
┌─────────────────────────┐
│     turul-mcp-server    │  ← High-level framework (this crate)
├─────────────────────────┤
│  turul-http-mcp-server  │  ← HTTP transport layer
├─────────────────────────┤
│ turul-mcp-json-rpc-server │ ← JSON-RPC dispatch
├─────────────────────────┤
│  turul-mcp-protocol     │  ← Protocol types & traits
└─────────────────────────┘
```

### Trait-Based Design

All MCP components use consistent trait patterns:

- **Tools** → `ToolDefinition` trait with fine-grained composition
- **Resources** → `ResourceDefinition` trait
- **Prompts** → `PromptDefinition` trait
- **Notifications** → `NotificationDefinition` trait

## Feature Flags

```toml
[dependencies]
turul-mcp-server = { version = "0.1", features = ["sqlite", "postgres"] }
```

- `default` - All features enabled
- `http` - HTTP transport layer (included by default)
- `sse` - Server-Sent Events streaming (included by default) 
- `sqlite` - SQLite session storage backend
- `postgres` - PostgreSQL session storage backend
- `dynamodb` - DynamoDB session storage backend

## Testing

```bash
# Run all tests
cargo test --package turul-mcp-server

# Test with specific storage backend
cargo test --package turul-mcp-server --features sqlite

# Integration tests
cargo test --package turul-mcp-server --test integration
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Contributing

See the main repository [CONTRIBUTING.md](../../CONTRIBUTING.md) for contribution guidelines.