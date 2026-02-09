# turul-mcp-client

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-client.svg)](https://crates.io/crates/turul-mcp-client)
[![Documentation](https://docs.rs/turul-mcp-client/badge.svg)](https://docs.rs/turul-mcp-client)

MCP client library with multi-transport support and full MCP 2025-11-25 protocol compliance.

## Overview

`turul-mcp-client` provides a complete client implementation for the Model Context Protocol (MCP), supporting multiple transport layers and offering both high-level and low-level APIs for interacting with MCP servers.

## Features

- ✅ **Multi-Transport Support** - HTTP and SSE transports
- ✅ **MCP 2025-11-25 Compliance** - Full protocol specification support
- ✅ **Session Management** - Automatic session handling with recovery
- ✅ **Streaming Support** - Real-time event streaming and progress tracking
- ✅ **Async/Await** - Built on Tokio for high performance
- ✅ **Error Recovery** - Comprehensive error types and retry mechanisms

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-client = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic HTTP Client

```rust
use turul_mcp_client::{McpClient, McpClientBuilder};
use turul_mcp_client::transport::HttpTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create HTTP transport
    let transport = HttpTransport::new("http://localhost:8080/mcp")?;
    
    // Build client
    let client = McpClientBuilder::new()
        .with_transport(Box::new(transport))
        .build();

    // Connect and initialize
    client.connect().await?;
    
    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {}", tools.len());
    
    // Call a tool
    let result = client.call_tool("calculator", serde_json::json!({
        "operation": "add",
        "a": 5,
        "b": 3
    })).await?;
    
    println!("Tool result: {:?}", result);
    Ok(())
}
```

## Transport Types

### HTTP Transport (Streamable HTTP)

For modern MCP servers supporting MCP 2025-11-25:

```rust
use turul_mcp_client::transport::HttpTransport;

let transport = HttpTransport::new("http://localhost:8080/mcp")?;

let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .build();
```

### SSE Transport (HTTP+SSE)

For servers supporting server-sent events:

```rust
use turul_mcp_client::transport::SseTransport;

let transport = SseTransport::new("http://localhost:8080/mcp")?;

let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .build();
```

### Future Transport Support

Additional transport implementations (stdio) are planned for future releases.

## Client Configuration

### Using ClientConfig

```rust
use turul_mcp_client::{McpClientBuilder, ClientConfig, RetryConfig, TimeoutConfig};
use std::time::Duration;

let config = ClientConfig {
    client_info: ClientInfo {
        name: "My MCP Client".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Custom MCP client".to_string()),
        vendor: None,
        metadata: None,
    },
    timeouts: TimeoutConfig {
        connect: Duration::from_secs(10),
        request: Duration::from_secs(30),
        long_operation: Duration::from_secs(120),
        initialization: Duration::from_secs(15),
        heartbeat: Duration::from_secs(30),
    },
    retry: RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(5),
        backoff_multiplier: 2.0,
        jitter: 0.1,
        exponential_backoff: true,
    },
    connection: ConnectionConfig::default(),
    logging: LoggingConfig::default(),
        request_timeout: Duration::from_secs(30),
    },
};

let client = McpClientBuilder::new()
    .with_config(config)
    .with_url("http://localhost:8080/mcp")?
    .build();
```

### Using URL Builder

```rust
let client = McpClientBuilder::new()
    .with_url("http://localhost:8080/mcp")?  // Automatically detects transport type
    .build();
```

## Session Management

### Connection Status

```rust
use turul_mcp_client::session::SessionState;

// Check connection and session status
let status = client.connection_status().await;
println!("Transport connected: {}", status.transport_connected);
println!("Session state: {:?}", status.session_state);
println!("Transport type: {}", status.transport_type);

if let Some(session_id) = status.session_id {
    println!("Session ID: {}", session_id);
}
```

### Session Information

```rust
// Get detailed session information
let session_info = client.session_info().await;
println!("Session ID: {:?}", session_info.session_id);
println!("Created: {:?}", session_info.created_at);
println!("State: {:?}", session_info.state);
```

### Connection Management

```rust
// Check if client is ready
if !client.is_ready().await {
    client.connect().await?;
}

// Disconnect and cleanup
client.disconnect().await?;
```

## Error Handling

### Error Types

```rust
use turul_mcp_client::{McpClientError, McpClientResult};

async fn robust_operation(client: &McpClient) -> McpClientResult<()> {
    match client.call_tool("my_tool", serde_json::json!({"param": "value"})).await {
        Ok(result) => {
            println!("Success: {:?}", result);
            Ok(())
        }
        Err(McpClientError::Transport(e)) => {
            tracing::warn!("Transport error, attempting reconnect: {}", e);
            client.disconnect().await?;
            client.connect().await?;
            Err(e.into())
        }
        Err(McpClientError::Session(e)) => {
            tracing::error!("Session error: {}", e);
            Err(e.into())
        }
        Err(McpClientError::Protocol(e)) => {
            tracing::error!("Protocol error: {}", e);
            Err(e.into())
        }
        Err(e) => Err(e),
    }
}
```

## Core Operations

### Tools

```rust
// List available tools
let tools = client.list_tools().await?;
for tool in &tools {
    println!("Tool: {}", tool.name);
    if let Some(description) = &tool.description {
        println!("  Description: {}", description);
    }
}

// Call a tool
let result = client.call_tool("calculator", serde_json::json!({
    "operation": "multiply",
    "a": 7,
    "b": 6
})).await?;

println!("Tool result: {:?}", result.content);
```

### Resources

```rust
// List available resources
let resources = client.list_resources().await?;
for resource in &resources {
    println!("Resource: {}", resource.uri);
    if let Some(description) = &resource.description {
        println!("  Description: {}", description);
    }
}

// Read a resource
let content = client.read_resource("file:///path/to/file.txt").await?;
println!("Resource content: {:?}", content);
```

### Prompts

```rust
// List available prompts
let prompts = client.list_prompts().await?;
for prompt in &prompts {
    println!("Prompt: {}", prompt.name);
    if let Some(description) = &prompt.description {
        println!("  Description: {}", description);
    }
}

// Get a prompt with arguments
let prompt_result = client.get_prompt("greeting", Some(serde_json::json!({
    "name": "Alice"
}))).await?;

println!("Prompt messages: {:?}", prompt_result.messages);
```

## Streaming and Events

### Stream Handler

```rust
// Access the stream handler for server events
let stream_handler = client.stream_handler().await;

// The stream handler processes server-sent events automatically
// This is primarily for internal use and advanced scenarios
```

## Protocol Headers

### MCP Protocol Version

The client automatically sends the appropriate protocol version header:

```rust
// Client automatically sends: MCP-Protocol-Version: 2025-11-25
// Server responds with: mcp-session-id: <session-uuid>

// Access session ID from connection status
let status = client.connection_status().await;
if let Some(session_id) = status.session_id {
    println!("Session ID from server: {}", session_id);
}
```

## Testing and Development

### Health Check

```rust
// Ping the server to check connectivity
match client.ping().await {
    Ok(_) => println!("Server is responsive"),
    Err(e) => println!("Server ping failed: {}", e),
}
```

### Transport Statistics

```rust
// Get transport layer statistics
let stats = client.transport_stats().await;
println!("Requests sent: {}", stats.requests_sent);
println!("Responses received: {}", stats.responses_received);
println!("Average response time: {:.2}ms", stats.avg_response_time_ms);
```

## Transport Detection

### Automatic Transport Selection

```rust
use turul_mcp_client::transport::{TransportFactory, detect_transport_type};

// Detect transport type from URL
let transport_type = detect_transport_type("http://localhost:8080/mcp")?;
println!("Detected transport: {}", transport_type);

// Create transport automatically
let transport = TransportFactory::from_url("http://localhost:8080/mcp")?;

// List available transports
let available = TransportFactory::available_transports();
println!("Available transports: {:?}", available);
```

## Examples

### Complete Application

```rust
use turul_mcp_client::{McpClient, McpClientBuilder, ClientConfig};
use turul_mcp_client::transport::HttpTransport;
use tracing::{info, error};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create client with configuration
    let mut config = ClientConfig::default();
    config.timeouts.connect = Duration::from_secs(10);
    config.timeouts.request = Duration::from_secs(30);
    
    let transport = HttpTransport::new("http://localhost:8080/mcp")?;
    let client = McpClientBuilder::new()
        .with_transport(Box::new(transport))
        .with_config(config)
        .build();
    
    // Connect
    info!("Connecting to MCP server...");
    client.connect().await?;
    info!("Connected successfully!");
    
    // Check if ready
    if !client.is_ready().await {
        error!("Client not ready after connect");
        return Ok(());
    }
    
    // Discover capabilities
    let tools = client.list_tools().await?;
    info!("Server provides {} tools", tools.len());
    
    for tool in &tools {
        info!("  - {}: {}", tool.name, 
              tool.description.as_deref().unwrap_or("No description"));
    }
    
    // Use first available tool
    if !tools.is_empty() {
        let tool_name = &tools[0].name;
        info!("Calling tool: {}", tool_name);
        
        match client.call_tool(tool_name, serde_json::json!({})).await {
            Ok(result) => {
                info!("Tool result: {:?}", result.content);
            }
            Err(e) => {
                error!("Tool call failed: {}", e);
            }
        }
    }
    
    // Get connection status
    let status = client.connection_status().await;
    info!("Connection status: transport_connected={}, session_state={:?}", 
          status.transport_connected, status.session_state);
    
    // Graceful cleanup
    info!("Disconnecting...");
    client.disconnect().await?;
    
    Ok(())
}
```

### Transport Comparison

```rust
use turul_mcp_client::transport::{HttpTransport, SseTransport, TransportCapabilities};

// Compare transport capabilities
fn compare_transports() -> Result<(), Box<dyn std::error::Error>> {
    let http_transport = HttpTransport::new("http://localhost:8080/mcp")?;
    let sse_transport = SseTransport::new("http://localhost:8080/mcp")?;
    
    let http_caps = http_transport.capabilities();
    let sse_caps = sse_transport.capabilities();
    
    println!("HTTP - Streaming: {}, Server Events: {}", 
             http_caps.streaming, http_caps.server_events);
    println!("SSE - Streaming: {}, Server Events: {}", 
             sse_caps.streaming, sse_caps.server_events);
             
    Ok(())
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-client = { version = "0.2", features = ["sse"] }
```

Available features:
- `default` = `["http", "sse"]` - HTTP and SSE transport
- `http` - HTTP transport support (included by default)
- `sse` - Server-Sent Events transport (included by default)
- `stdio` - *(Planned)* Standard I/O transport for executable servers

## Error Reference

### McpClientError Types

```rust
use turul_mcp_client::McpClientError;

match error {
    McpClientError::Transport(e) => {
        // Network/transport related errors
        eprintln!("Transport error: {}", e);
    }
    McpClientError::Protocol(e) => {
        // MCP protocol violations or incompatibilities
        eprintln!("Protocol error: {}", e);
    }
    McpClientError::Session(e) => {
        // Session management errors
        eprintln!("Session error: {}", e);
    }
    McpClientError::Timeout => {
        // Request timeout
        eprintln!("Request timed out");
    }
    McpClientError::NotConnected => {
        // Client not connected
        eprintln!("Client not connected");
    }
    McpClientError::InvalidResponse(msg) => {
        // Invalid response from server
        eprintln!("Invalid response: {}", msg);
    }
}
```

## Performance Notes

- **Connection Reuse**: Transport connections are reused across requests
- **Async/Await**: All operations are non-blocking and async
- **Memory Efficient**: Streaming responses avoid large memory allocations
- **Session Cleanup**: Automatic session cleanup on client drop

## Compatibility

### MCP Protocol Versions

The client automatically adapts to server capabilities:

- **2024-11-05**: Basic MCP without streamable HTTP
- **2025-03-26**: Streamable HTTP with SSE support  
- **2025-06-18**: Full feature set with meta fields and enhanced capabilities

### Transport Compatibility

- **HTTP**: Works with all MCP servers
- **SSE**: Requires server-sent events support
- **Stdio**: *(Planned)* Executable MCP server support

## Related Crates

- **[turul-mcp-server](../turul-mcp-server)**: Complete MCP server framework
- **[turul-mcp-protocol](../turul-mcp-protocol)**: MCP protocol types and traits
- **[turul-http-mcp-server](../turul-http-mcp-server)**: HTTP transport layer for servers

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
