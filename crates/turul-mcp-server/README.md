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

## Architectural Patterns

This framework supports two primary architectural patterns for building MCP servers:

1. **Simple (Integrated Transport):** This is the easiest way to get started. The `McpServer` includes a default HTTP transport layer. You can configure and run the server with a single builder chain, as shown in the Quick Start examples below. This is recommended for most use cases.

2. **Advanced (Pluggable Transport):** For more complex scenarios, such as serverless deployments or custom transports, you can use `McpServer::builder()` to create a transport-agnostic configuration object. This object is then passed to a separate transport crate, like `turul-mcp-aws-lambda` or `turul-http-mcp-server`, for execution. This provides maximum flexibility.

See the `turul-http-mcp-server` README for a detailed example of the pluggable transport pattern.

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
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add) // Use original function name
        .bind_address("127.0.0.1:8641".parse()?)  // Default port
        .build()?;

    server.run().await
}
```

### Level 2: Derive Macros (Struct-Based)
*Requires: `turul-mcp-derive = "0.2.0"` dependency*

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

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
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8641".parse()?)  // Default port
        .build()?;

    server.run().await
}
```

### Level 3: Builder Pattern (Runtime Flexibility)

**Note**: `ToolBuilder` is re-exported from the server crate for convenience. You can use either:
- `turul_mcp_server::ToolBuilder` (recommended for servers)  
- `turul_mcp_builders::ToolBuilder` (direct import)

```rust
use turul_mcp_server::prelude::*;
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

    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(calculator)
        .bind_address("127.0.0.1:8641".parse()?)  // Default port
        .build()?;

    server.run().await
}
```

## Resources

The framework provides powerful resource management with automatic URI template detection and parameter extraction.

### Resource Function Registration

Use `.resource_fn()` to register resources created with the `#[mcp_resource]` macro:

```rust
use turul_mcp_derive::mcp_resource;
use turul_mcp_server::prelude::*;
use turul_mcp_protocol::resources::ResourceContent;

// Static resource
#[mcp_resource(
    uri = "file:///config.json",
    name = "config",
    description = "Application configuration"
)]
async fn get_config() -> McpResult<Vec<ResourceContent>> {
    let config = serde_json::json!({
        "app_name": "My Server",
        "version": "1.0.0",
        "debug": true
    });

    Ok(vec![ResourceContent::blob(
        "file:///config.json",
        serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
        "application/json".to_string()
    )])
}

// Template resource - automatic parameter extraction
#[mcp_resource(
    uri = "file:///users/{user_id}.json",
    name = "user_profile",
    description = "User profile data"
)]
async fn get_user_profile(user_id: String) -> McpResult<Vec<ResourceContent>> {
    let profile = serde_json::json!({
        "user_id": user_id,
        "username": format!("user_{}", user_id),
        "email": format!("{}@example.com", user_id)
    });

    Ok(vec![ResourceContent::blob(
        format!("file:///users/{}.json", user_id),
        serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?,
        "application/json".to_string()
    )])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("resource-server")
        .version("1.0.0")
        .resource_fn(get_config)       // Static resource
        .resource_fn(get_user_profile) // Template: file:///users/{user_id}.json
        .bind_address("127.0.0.1:8641".parse()?)  // Default port
        .build()?;

    // Framework automatically:
    // - Detects URI templates ({user_id} patterns)
    // - Registers appropriate resource handlers
    // - Extracts template variables from requests
    // - Maps them to function parameters
    Ok(())
}
```

### Alternative: Direct Resource Registration

You can also register resource instances directly:

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_protocol::resources::*;
use async_trait::async_trait;

struct ConfigResource;

#[async_trait]
impl McpResource for ConfigResource {
    async fn read(&self, _params: Option<serde_json::Value>) -> McpResult<Vec<ResourceContent>> {
        // Custom implementation
        Ok(vec![])
    }
}

// Implement metadata traits...

let server = McpServer::builder()
    .resource(ConfigResource)  // Direct instance
    .build()?;
```

## Session Management & Storage

### Pluggable Storage Backends

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::{SqliteSessionStorage, PostgresSessionStorage};
use std::sync::Arc;

// SQLite for single-instance deployments
let storage = Arc::new(SqliteSessionStorage::new().await?);

// PostgreSQL for multi-instance deployments
let storage = Arc::new(PostgresSessionStorage::new().await?);

let server = McpServer::builder()
    .name("postgres-server")
    .version("1.0.0")
    .with_session_storage(storage)
    .bind_address("127.0.0.1:8080".parse()?)
    // Add your tools here: .tool(your_tool)
    .build()?;

server.run().await
```

### Session Context in Tools

```rust
use turul_mcp_server::prelude::*;
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
        let current: i32 = session.get_typed_state("counter").await.unwrap_or(0);
            let new_count = current + 1;
            
            // Save updated counter
            session.set_typed_state("counter", new_count).await
                .map_err(|e| format!("Failed to save state: {}", e))?;
            
            // Send progress notification
            session.notify_progress("counting", new_count as u64).await;
            
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
session.notify_progress("task-123", 75).await;

session.notify_log(LoggingLevel::Info, serde_json::json!({"message": "Operation completed successfully"}), None, None).await;
```

## Server Configuration

### HTTP Server with Custom Port

```rust
use turul_mcp_server::prelude::*;

let server = McpServer::builder()
    .name("my-server")
    .version("1.0.0")
    .bind_address("127.0.0.1:8080".parse()?)  // Custom bind address
    // Add your tools here: .tool(your_tool)
    .build()?;

server.run().await
```

### Production Configuration

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_session_storage::PostgresSessionStorage;
use std::sync::Arc;

let storage = Arc::new(PostgresSessionStorage::new().await?);

let server = McpServer::builder()
    .name("production-server")
    .version("1.0.0")
    .bind_address("0.0.0.0:3000".parse()?)
    .with_session_storage(storage)
    // Add your tools here: .tool(your_tool)
    .build()?;

server.run().await
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
let server = McpServer::builder()
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

All MCP components use consistent trait patterns from `turul-mcp-builders`:

- **Tools** → `ToolDefinition` trait with fine-grained composition
- **Resources** → `ResourceDefinition` trait
- **Prompts** → `PromptDefinition` trait
- **Notifications** → `NotificationDefinition` trait

See [`turul-mcp-builders`](../turul-mcp-builders/README.md) for trait documentation.

## Feature Flags

```toml
[dependencies]
turul-mcp-server = { version = "0.2", features = ["sqlite", "postgres"] }
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
