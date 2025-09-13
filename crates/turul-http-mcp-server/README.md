# turul-http-mcp-server

[![Crates.io](https://img.shields.io/crates/v/turul-http-mcp-server.svg)](https://crates.io/crates/turul-http-mcp-server)
[![Documentation](https://docs.rs/turul-http-mcp-server/badge.svg)](https://docs.rs/turul-http-mcp-server)

HTTP transport layer components for Model Context Protocol (MCP) servers with full MCP 2025-06-18 Streamable HTTP compliance and SSE support.

## Overview

`turul-http-mcp-server` provides the foundational HTTP transport components used by `turul-mcp-server`. It implements MCP Streamable HTTP with Server-Sent Events (SSE) for real-time notifications and session management.

**⚠️ Important**: Most users should use `turul-mcp-server` directly, which includes this transport layer automatically. This crate is primarily for internal framework use and advanced customization scenarios.

**Recommended approach**: Use `turul_mcp_server::McpServerBuilder` which handles HTTP transport configuration internally.

## Features

- ✅ **MCP 2025-06-18 Streamable HTTP** - Full protocol compliance with SSE streaming
- ✅ **Session Management** - UUID v7 session IDs with automatic cleanup
- ✅ **SSE Resumability** - Last-Event-ID support with event replay
- ✅ **CORS Support** - Browser client compatibility with configurable origins
- ✅ **Protocol Version Detection** - Automatic feature flags based on client capabilities
- ✅ **JSON-RPC Dispatch** - Efficient method routing and error handling

## Architecture

### Transport Layer Components

This crate provides the building blocks used by the main server:

```
┌─────────────────────────────────────────────────┐
│                MCP Client                       │
├─────────────────────────────────────────────────┤
│  POST /mcp + Accept: application/json           │  ← JSON-RPC requests
│  GET  /mcp + Accept: text/event-stream          │  ← SSE notifications
├─────────────────────────────────────────────────┤
│          turul-http-mcp-server                  │  ← This crate
│  ├─ SessionMcpHandler                          │  ← Session management
│  ├─ StreamManager                              │  ← SSE event streaming  
│  ├─ NotificationBroadcaster                    │  ← Real-time notifications
│  └─ JsonRpcDispatcher                          │  ← JSON-RPC routing
├─────────────────────────────────────────────────┤
│            turul-mcp-server                     │  ← Main framework
└─────────────────────────────────────────────────┘
```

### Core Components

```rust
use turul_http_mcp_server::{
    // HTTP server builder and configuration
    HttpMcpServerBuilder, ServerConfig,
    
    // Session and stream management
    SessionMcpHandler, StreamManager, StreamConfig,
    
    // Notifications and CORS
    NotificationBroadcaster, CorsLayer,
    
    // JSON-RPC dispatch
    JsonRpcDispatcher, JsonRpcHandler,
};
```

## Usage

### Advanced: Direct Transport Configuration

**⚠️ For advanced use cases only. Most users should use `turul-mcp-server` instead.**

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, ServerConfig};
use turul_mcp_session_storage::InMemorySessionStorage;
use std::net::SocketAddr;
use std::sync::Arc;

// Advanced: Direct HTTP transport configuration
// This is for custom scenarios where you need direct control over the transport layer
let transport = HttpMcpServerBuilder::new()
    .bind_address("127.0.0.1:3000".parse()?)
    .mcp_path("/mcp")
    .cors(true)
    .sse(true)
    .session_expiry_minutes(30)
    .build();

// Note: This only creates the transport layer. You'll need to integrate it 
// with your own application logic and MCP message handling.
```

### Session Management Configuration

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, SessionMcpHandler};
use turul_mcp_session_storage::InMemorySessionStorage;
use std::sync::Arc;

// Configure session storage
let storage = Arc::new(InMemorySessionStorage::new());

let server = HttpMcpServerBuilder::with_storage(storage)
    .bind_address("0.0.0.0:3000".parse()?)
    .session_expiry_minutes(30)
    .build();
```

### SSE Stream Configuration

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, StreamConfig};

let stream_config = StreamConfig {
    buffer_size: 1000,
    event_replay_limit: 100,
    heartbeat_interval_seconds: 30,
    max_concurrent_streams: 1000,
};

let server = HttpMcpServerBuilder::new()
    .stream_config(stream_config)
    .get_sse(true)  // Enable GET SSE for notifications
    .post_sse(false) // Disable POST SSE for compatibility
    .build();
```

### CORS Configuration

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, CorsLayer};

// Simple CORS enablement
let server = HttpMcpServerBuilder::new()
    .cors(true)  // Uses permissive defaults for development
    .build();

// Custom CORS configuration (Note: CorsLayer configuration
// is handled internally - this crate provides the components
// but turul-mcp-server provides the full API)
```

### JSON-RPC Handler Registration

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, JsonRpcHandler};
use async_trait::async_trait;

// Custom handler implementation
struct CustomHandler;

#[async_trait]
impl JsonRpcHandler for CustomHandler {
    async fn handle(
        &self, 
        method: &str, 
        params: Option<turul_mcp_json_rpc_server::RequestParams>,
        session_context: Option<turul_mcp_json_rpc_server::SessionContext>
    ) -> turul_mcp_json_rpc_server::JsonRpcResult<serde_json::Value> {
        match method {
            "custom/method" => Ok(serde_json::json!({"result": "success"})),
            _ => Err(turul_mcp_json_rpc_server::JsonRpcProcessingError::MethodNotFound(method.to_string()).into()),
        }
    }
}

// Note: This crate provides transport layer components.
// For full server functionality including handler registration,
// use turul-mcp-server which builds on this transport layer.
let server = HttpMcpServerBuilder::new()
    .build();
```

## Protocol Version Detection

The transport layer automatically detects client capabilities:

```rust
use turul_http_mcp_server::{
    extract_protocol_version, McpProtocolVersion
};

// Protocol version extraction from headers
let version = extract_protocol_version(&headers);
match version {
    McpProtocolVersion::V2024_11_05 => {
        // Basic MCP without streamable HTTP
    }
    McpProtocolVersion::V2025_03_26 => {
        // Streamable HTTP support
    }
    McpProtocolVersion::V2025_06_18 => {
        // Full feature set with _meta, cursor, progressToken
    }
}
```

## Session Management

### Session ID Extraction

```rust
use turul_http_mcp_server::{extract_session_id, SessionMcpHandler};

// Extract session ID from request headers
let session_id = extract_session_id(&headers);

// Session handler for managing session lifecycle
let handler = SessionMcpHandler::new(
    session_storage,
    stream_manager,
    json_rpc_dispatcher
);
```

### Session Storage Integration

```rust
use turul_http_mcp_server::HttpMcpServerBuilder;
use turul_mcp_session_storage::{InMemorySessionStorage, SqliteSessionStorage};
use std::sync::Arc;

// In-memory storage (development)
let memory_storage = Arc::new(InMemorySessionStorage::new());
let server = HttpMcpServerBuilder::with_storage(memory_storage).build();

// SQLite storage (production)
#[cfg(feature = "sqlite")]
{
    let sqlite_storage = Arc::new(SqliteSessionStorage::new("sessions.db").await?);
    let server = HttpMcpServerBuilder::with_storage(sqlite_storage).build();
}
```

## Notification Broadcasting

### SSE Event Streaming

```rust
use turul_http_mcp_server::{
    NotificationBroadcaster, StreamManager, 
    StreamManagerNotificationBroadcaster
};
use std::sync::Arc;

// Create notification broadcaster
let stream_manager = Arc::new(StreamManager::new(session_storage));
let broadcaster = StreamManagerNotificationBroadcaster::new(stream_manager);

// Send notifications to specific sessions
broadcaster.notify_session(
    "session-123", 
    serde_json::json!({
        "method": "notifications/progress",
        "params": {
            "progressToken": "task-456",
            "progress": 75,
            "total": 100
        }
    })
).await?;
```

### Event Replay and Resumability

```rust
use turul_http_mcp_server::{extract_last_event_id, StreamManager};

// Extract Last-Event-ID for resumability
let last_event_id = extract_last_event_id(&headers);

// Stream manager handles event replay automatically
let stream = stream_manager.create_stream(
    session_id,
    last_event_id  // Resume from this event
).await?;
```

## Protocol Headers

### MCP Header Handling

The transport layer automatically handles MCP-specific headers:

```rust
// Client sends: MCP-Protocol-Version: 2025-06-18
// Server returns: mcp-session-id: <uuid-v7>

// The transport layer extracts and processes these headers automatically
use turul_http_mcp_server::{extract_protocol_version, extract_session_id};

// Headers are processed internally by SessionMcpHandler
// Protocol version determines feature availability
// Session ID manages state isolation between clients
```

### Lifecycle Management

Optional strict lifecycle gating can be configured:

```rust
// Require notifications/initialized before allowing operations
let server = HttpMcpServerBuilder::new()
    .strict_lifecycle(true)  // Reject operations before initialize
    .build();
```

## Error Handling

### HTTP Transport Errors

```rust
use turul_http_mcp_server::{HttpMcpError, Result};

fn handle_transport_error(error: HttpMcpError) {
    match error {
        HttpMcpError::Http(e) => {
            println!("HTTP error: {}", e);
        }
        HttpMcpError::JsonRpc(e) => {
            println!("JSON-RPC error: {}", e);
        }
        HttpMcpError::Mcp(e) => {
            println!("MCP protocol error: {}", e);
        }
        HttpMcpError::InvalidRequest(msg) => {
            println!("Invalid request: {}", msg);
        }
        _ => {
            println!("Other transport error: {}", error);
        }
    }
}
```

## Server Statistics and Monitoring

```rust
use turul_http_mcp_server::{ServerStats, StreamStats};

// Server statistics (if implemented by the specific server)
// Note: Full stats API is available in turul-mcp-server
```

## Testing the Transport Layer

### Manual HTTP Testing

```bash
# Test session creation
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  -v  # Note the Mcp-Session-Id header in response

# Test SSE streaming
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://localhost:3000/mcp

# Test event resumability
curl -N -H "Accept: text/event-stream" \
  -H "Last-Event-ID: event-123" \
  -H "Mcp-Session-Id: <session-id>" \
  http://localhost:3000/mcp
```

### Integration Testing

```rust
use turul_http_mcp_server::{HttpMcpServerBuilder, ServerConfig};

#[tokio::test]
async fn test_transport_layer() {
    let server = HttpMcpServerBuilder::new()
        .bind_address("127.0.0.1:0".parse().unwrap()) // Random port
        .build();
    
    // Test server configuration
    assert!(server.config.enable_cors);
    assert_eq!(server.config.mcp_path, "/mcp");
}
```

## Framework Integration

### ✅ Recommended: Using turul-mcp-server

**This is the recommended approach for most users:**

```rust
// Recommended: Use the main server framework
use turul_mcp_server::McpServerBuilder;

let server = McpServerBuilder::new()
    .name("My Server")
    .version("1.0.0")
    .bind_address("127.0.0.1:3000".parse()?)
    .build()?;

// Note: Server configuration complete - HTTP transport layer is included
// Refer to turul-mcp-server docs for deployment patterns
// from turul-http-mcp-server with sensible defaults
```

### Advanced Transport Customization

```rust
// Advanced: Direct transport layer usage for custom scenarios
use turul_http_mcp_server::{HttpMcpServerBuilder, SessionMcpHandler};
use turul_mcp_session_storage::InMemorySessionStorage;

// Build custom HTTP transport
let transport = HttpMcpServerBuilder::new()
    .bind_address("127.0.0.1:3000".parse()?)
    .session_expiry_minutes(60)
    .max_body_size(2 * 1024 * 1024)
    .build();

// Integrate with custom application logic
```

## Feature Flags

```toml
[dependencies]
turul-http-mcp-server = { version = "0.2.0", features = ["sse"] }
```

Available features:
- `default` = `["sse"]` - Includes SSE support by default
- `sse` - Server-Sent Events streaming for real-time notifications

## Performance Notes

- **Connection Handling**: Uses Hyper for efficient HTTP/1.1 connections
- **Stream Management**: Optimized SSE event delivery with configurable buffers
- **Session Cleanup**: Automatic cleanup every 60 seconds with configurable expiry
- **JSON-RPC Dispatch**: Fast method routing with minimal allocations

## Compatibility

### MCP Protocol Versions

This transport layer supports all MCP protocol versions:

- **2024-11-05**: Basic MCP without streamable HTTP
- **2025-03-26**: Streamable HTTP with SSE support  
- **2025-06-18**: Full feature set with meta fields and enhanced capabilities

### HTTP Specifications

- **HTTP/1.1**: Full support with connection keep-alive
- **Server-Sent Events**: Compliant with EventSource specification
- **CORS**: Cross-Origin Resource Sharing for browser clients
- **JSON-RPC 2.0**: Complete specification compliance

## Related Crates

- **[turul-mcp-server](../turul-mcp-server)**: Complete MCP server framework (recommended for most users)
- **[turul-mcp-session-storage](../turul-mcp-session-storage)**: Pluggable session storage backends
- **[turul-mcp-protocol](../turul-mcp-protocol)**: MCP protocol types and traits
- **[turul-mcp-json-rpc-server](../turul-mcp-json-rpc-server)**: JSON-RPC 2.0 server foundation

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.