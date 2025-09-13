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
turul-mcp-server = "0.2.0"
turul-mcp-derive = "0.2.0"  # Required for function macros and derive macros
tokio = { version = "1.0", features = ["full"] }
```

### Level 1: Function Macros (Simplest)

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpServerBuilder, McpResult};

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServerBuilder::new()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add) // Use original function name
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    // Note: Server configuration complete - refer to turul-http-mcp-server for HTTP transport
    // or turul-mcp-aws-lambda for Lambda deployment
}
```

### Level 2: Derive Macros (Struct-Based)
*Requires: `turul-mcp-derive = "0.2.0"` dependency*

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpServerBuilder, McpResult, SessionContext};

#[derive(McpTool, Clone)]
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
    let server = McpServerBuilder::new()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    // Note: Server configuration complete - refer to turul-http-mcp-server for HTTP transport
    // or turul-mcp-aws-lambda for Lambda deployment
}
```

### Level 3: Builder Pattern (Runtime Flexibility)

**Note**: `ToolBuilder` is re-exported from the server crate for convenience. You can use either:
- `turul_mcp_server::ToolBuilder` (recommended for servers)  
- `turul_mcp_builders::ToolBuilder` (direct import)

```rust
use turul_mcp_server::McpServerBuilder;
use turul_mcp_builders::ToolBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calculator = ToolBuilder::new("calculator")
        .description("Add two numbers")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output() // Generates {"result": number} schema
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'a'")?;
            let b = args.get("b").and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'b'")?;
            
            Ok(json!({"result": a + b}))
        })
        .build()
        .map_err(|e| format!("Failed to build tool: {}", e))?;

    let server = McpServerBuilder::new()
        .name("calculator-server")
        .version("1.0.0")
        .tool(calculator)
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    // Note: Server configuration complete - refer to turul-http-mcp-server for HTTP transport
    // or turul-mcp-aws-lambda for Lambda deployment
}
```

## Session Management & Storage

### Pluggable Storage Backends

```rust
use turul_mcp_server::McpServerBuilder;
use turul_mcp_session_storage::{SqliteSessionStorage, PostgreSqlSessionStorage};
use std::sync::Arc;

// SQLite for single-instance deployments
let storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);

// PostgreSQL for multi-instance deployments  
let storage = Arc::new(PostgresSessionStorage::new("postgresql://...").await?);

let server = McpServerBuilder::new()
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
struct StatefulCounter {
    // Derive macros require named fields, so we add a dummy field
    _marker: (),
}

impl StatefulCounter {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<i32> {
        if let Some(session) = session {
            // Get current counter or start at 0
            let current: i32 = session.get_typed_state("counter").unwrap_or(0);
            let new_count = current + 1;
            
            // Save updated counter
            session.set_typed_state("counter", new_count)
                .map_err(|e| format!("Failed to save state: {}", e))?;
            
            // Send progress notification
            session.notify_progress("counting", new_count as u64);
            
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
session.notify_progress("task-123", 75);

session.notify_log("info", "Operation completed successfully");
```

## Server Configuration

### HTTP Server with Custom Port

```rust
use turul_mcp_server::McpServerBuilder;

let server = McpServerBuilder::new()
    .name("my-server")
    .version("1.0.0")
    .bind_address("127.0.0.1:8080".parse()?)  // Custom bind address
    .tool(/* your tools */)
    .build()?;

// Note: Server configuration complete - refer to turul-http-mcp-server for HTTP transport
// or turul-mcp-aws-lambda for Lambda deployment
Ok(())
```

### Production Configuration

```rust
use turul_mcp_server::McpServerBuilder;
use turul_mcp_session_storage::PostgresSessionStorage;
use std::sync::Arc;

let storage = Arc::new(PostgresSessionStorage::new(
    std::env::var("DATABASE_URL")?
).await?);

let server = McpServerBuilder::new()
    .name("production-server")
    .version("1.0.0") 
    .bind_address("0.0.0.0:3000".parse()?)
    .with_session_storage(storage)
    .tool(/* your tools */)
    .build()?;

// Note: Server configuration complete - refer to turul-http-mcp-server for HTTP transport
// or turul-mcp-aws-lambda for Lambda deployment
Ok(())
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

### Server Capabilities

Static servers should advertise truthful capabilities:

```rust
let server = McpServerBuilder::new()
    .name("static-server")
    .version("1.0.0")
    .tool(calculator)
    .build()?;

// Framework automatically sets:
// - tools.listChanged = false (static tool list)
// - prompts.listChanged = false (static prompt list)  
// - resources.subscribe = false (no subscriptions)
// - resources.listChanged = false (static resource list)

// Optional strict lifecycle: notifications/initialized can be sent after successful setup
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
turul-mcp-server = { version = "0.2.0", features = ["sqlite", "postgres"] }
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