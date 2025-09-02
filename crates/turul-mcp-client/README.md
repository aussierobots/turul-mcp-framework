# turul-mcp-client

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-client.svg)](https://crates.io/crates/turul-mcp-client)
[![Documentation](https://docs.rs/turul-mcp-client/badge.svg)](https://docs.rs/turul-mcp-client)

Comprehensive MCP client library with multi-transport support and full MCP 2025-06-18 protocol compliance.

## Overview

`turul-mcp-client` provides a complete client implementation for the Model Context Protocol (MCP), supporting multiple transport layers and offering both high-level and low-level APIs for interacting with MCP servers.

## Features

- ✅ **Multi-Transport Support** - HTTP, SSE, WebSocket, and stdio transports
- ✅ **MCP 2025-06-18 Compliance** - Full protocol specification support
- ✅ **Session Management** - Automatic session handling with recovery
- ✅ **Streaming Support** - Real-time event streaming and progress tracking
- ✅ **Async/Await** - Built on Tokio for high performance
- ✅ **Error Recovery** - Comprehensive error types and retry mechanisms
- ✅ **Connection Pooling** - Efficient resource management

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-client = "0.1"
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
        .with_transport(transport)
        .with_client_info("My MCP Client", "1.0.0")
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

For modern MCP servers supporting MCP 2025-06-18:

```rust
use turul_mcp_client::transport::HttpTransport;

let transport = HttpTransport::new("http://localhost:8080/mcp")?
    .with_timeout(std::time::Duration::from_secs(30))
    .with_retry_config(RetryConfig::default());

let client = McpClientBuilder::new()
    .with_transport(transport)
    .build();
```

### SSE Transport (HTTP+SSE)

For servers supporting server-sent events:

```rust
use turul_mcp_client::transport::SseTransport;

let transport = SseTransport::new("http://localhost:8080/mcp")?
    .with_heartbeat_interval(std::time::Duration::from_secs(30));

let client = McpClientBuilder::new()
    .with_transport(transport)
    .enable_streaming()  // Enable real-time notifications
    .build();
```

### WebSocket Transport

For WebSocket-based MCP servers:

```rust
use turul_mcp_client::transport::WebSocketTransport;

let transport = WebSocketTransport::new("ws://localhost:8080/mcp")?
    .with_ping_interval(std::time::Duration::from_secs(30));

let client = McpClientBuilder::new()
    .with_transport(transport)
    .build();
```

### Stdio Transport

For command-line MCP server executables:

```rust
use turul_mcp_client::transport::StdioTransport;

let transport = StdioTransport::new("./my-mcp-server")?
    .with_args(vec!["--config", "config.json"])
    .with_working_dir("/path/to/server");

let client = McpClientBuilder::new()
    .with_transport(transport)
    .build();
```

## Advanced Usage

### Client Configuration

```rust
use turul_mcp_client::{McpClientBuilder, ClientConfig, RetryConfig, TimeoutConfig};

let config = ClientConfig {
    connect_timeout: std::time::Duration::from_secs(10),
    request_timeout: std::time::Duration::from_secs(30),
    retry_config: RetryConfig {
        max_retries: 3,
        initial_delay: std::time::Duration::from_millis(100),
        max_delay: std::time::Duration::from_secs(5),
        exponential_base: 2.0,
    },
    enable_streaming: true,
    buffer_size: 8192,
};

let client = McpClientBuilder::new()
    .with_config(config)
    .with_transport(transport)
    .build();
```

### Session Management

```rust
use turul_mcp_client::{McpClient, SessionState};

// Check session status
match client.session_state().await {
    SessionState::Connected { session_id } => {
        println!("Connected with session: {}", session_id);
    }
    SessionState::Disconnected => {
        println!("Not connected");
        client.connect().await?;
    }
    SessionState::Error { error } => {
        println!("Session error: {}", error);
        client.reconnect().await?;
    }
}

// Manual session management
let session_info = client.session_info().await?;
println!("Session ID: {}", session_info.session_id);
println!("Created: {}", session_info.created_at);
println!("Last activity: {}", session_info.last_activity);
```

### Error Handling

```rust
use turul_mcp_client::{McpClientError, McpClientResult};

async fn robust_tool_call(client: &McpClient) -> McpClientResult<serde_json::Value> {
    match client.call_tool("my_tool", serde_json::json!({"param": "value"})).await {
        Ok(result) => Ok(result),
        Err(McpClientError::Transport(e)) => {
            tracing::warn!("Transport error, retrying: {}", e);
            client.reconnect().await?;
            client.call_tool("my_tool", serde_json::json!({"param": "value"})).await
        }
        Err(McpClientError::Session(e)) => {
            tracing::error!("Session error: {}", e);
            client.disconnect().await?;
            client.connect().await?;
            client.call_tool("my_tool", serde_json::json!({"param": "value"})).await
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
for tool in tools {
    println!("Tool: {} - {}", tool.name, tool.description.unwrap_or_default());
}

// Call a tool
let result = client.call_tool("calculator", serde_json::json!({
    "operation": "multiply",
    "a": 7,
    "b": 6
})).await?;

println!("Result: {:?}", result.content);

// Call tool with progress tracking
let result = client.call_tool_with_progress("long_task", 
    serde_json::json!({"duration": 10}),
    |progress| {
        println!("Progress: {}%", progress.progress);
        if let Some(message) = &progress.message {
            println!("Status: {}", message);
        }
    }
).await?;
```

### Resources

```rust
// List available resources
let resources = client.list_resources().await?;
for resource in resources {
    println!("Resource: {} - {}", resource.uri, 
             resource.description.unwrap_or_default());
}

// Read a resource
let content = client.read_resource("file:///path/to/file.txt").await?;
println!("Resource content: {:?}", content);

// Subscribe to resource updates
client.subscribe_to_resource_updates(|notification| {
    println!("Resource updated: {}", notification.uri);
}).await?;
```

### Prompts

```rust
// List available prompts
let prompts = client.list_prompts().await?;
for prompt in prompts {
    println!("Prompt: {} - {}", prompt.name, 
             prompt.description.unwrap_or_default());
}

// Get a prompt
let prompt = client.get_prompt("greeting", Some(serde_json::json!({
    "name": "Alice"
}))).await?;

println!("Prompt messages:");
for message in prompt.messages {
    println!("  {}: {}", message.role, message.content);
}
```

### Sampling

```rust
// Create sampling request
let messages = vec![
    Message {
        role: Role::User,
        content: MessageContent::Text {
            text: "Hello, can you help me?".to_string(),
        },
    }
];

let sampling_result = client.create_message(messages, None, None).await?;
println!("Generated message: {:?}", sampling_result);
```

## Streaming and Events

### Real-time Notifications

```rust
use turul_mcp_client::streaming::EventHandler;

struct MyEventHandler;

impl EventHandler for MyEventHandler {
    async fn handle_progress(&self, progress: ProgressNotification) {
        println!("Progress: {}% - {}", 
                 progress.progress, 
                 progress.message.unwrap_or_default());
    }
    
    async fn handle_log(&self, log: LoggingNotification) {
        println!("Log [{}]: {:?}", log.level, log.data);
    }
    
    async fn handle_resource_updated(&self, notification: ResourceUpdatedNotification) {
        println!("Resource updated: {}", notification.uri);
    }
}

// Enable streaming with custom handler
let client = McpClientBuilder::new()
    .with_transport(transport)
    .with_event_handler(Box::new(MyEventHandler))
    .enable_streaming()
    .build();
```

### Progress Tracking

```rust
// Track progress for long-running operations
let progress_receiver = client.call_tool_with_progress_stream(
    "data_processing", 
    serde_json::json!({"file_path": "/large/dataset.csv"})
).await?;

// Handle progress updates
tokio::spawn(async move {
    while let Some(progress) = progress_receiver.recv().await {
        match progress {
            ProgressUpdate::Progress { progress, total, message } => {
                let percentage = progress / total.unwrap_or(1.0) * 100.0;
                println!("Processing: {:.1}% - {}", 
                         percentage, 
                         message.unwrap_or_default());
            }
            ProgressUpdate::Completed { result } => {
                println!("Processing completed: {:?}", result);
                break;
            }
            ProgressUpdate::Failed { error } => {
                println!("Processing failed: {}", error);
                break;
            }
        }
    }
});
```

## Testing Support

### Mock Transport

```rust
use turul_mcp_client::transport::MockTransport;

#[tokio::test]
async fn test_tool_calling() {
    let mut mock_transport = MockTransport::new();
    
    // Set up expected responses
    mock_transport.expect_initialize_response(InitializeResult {
        protocol_version: "2025-06-18".to_string(),
        capabilities: Default::default(),
        server_info: ServerInfo {
            name: "Test Server".to_string(),
            version: "1.0.0".to_string(),
        },
    });
    
    mock_transport.expect_call_tool_response("calculator", CallToolResult {
        content: vec![ToolResult::text("42")],
        is_error: false,
        _meta: None,
    });
    
    let client = McpClientBuilder::new()
        .with_transport(mock_transport)
        .build();
        
    client.connect().await.unwrap();
    
    let result = client.call_tool("calculator", 
        serde_json::json!({"operation": "add", "a": 20, "b": 22})).await.unwrap();
        
    assert_eq!(result.content[0].text.as_ref().unwrap(), "42");
}
```

### Integration Testing

```rust
use turul_mcp_client::testing::{TestServer, TestTransport};

#[tokio::test]
async fn integration_test() {
    // Start a test server
    let test_server = TestServer::new()
        .with_tool("echo", |args| async move {
            Ok(serde_json::json!({"echo": args}))
        })
        .start()
        .await?;
    
    // Connect client to test server
    let transport = HttpTransport::new(&test_server.url())?;
    let client = McpClientBuilder::new()
        .with_transport(transport)
        .build();
    
    client.connect().await?;
    
    // Test the interaction
    let result = client.call_tool("echo", 
        serde_json::json!({"message": "hello"})).await?;
        
    assert_eq!(result.content[0].text.as_ref().unwrap(), 
               r#"{"echo": {"message": "hello"}}"#);
}
```

## Performance Optimization

### Connection Pooling

```rust
use turul_mcp_client::{McpClientPool, PoolConfig};

let pool_config = PoolConfig {
    max_connections: 10,
    min_connections: 2,
    connection_timeout: std::time::Duration::from_secs(30),
    idle_timeout: std::time::Duration::from_secs(300),
};

let pool = McpClientPool::new(transport_factory, pool_config).await?;

// Get a client from the pool
let client = pool.get_client().await?;
let result = client.call_tool("my_tool", args).await?;

// Client automatically returns to pool when dropped
drop(client);
```

### Batch Operations

```rust
// Batch multiple tool calls
let batch_requests = vec![
    ("tool_1", serde_json::json!({"param": "value1"})),
    ("tool_2", serde_json::json!({"param": "value2"})),
    ("tool_3", serde_json::json!({"param": "value3"})),
];

let results = client.batch_call_tools(batch_requests).await?;
for (i, result) in results.into_iter().enumerate() {
    println!("Tool {} result: {:?}", i + 1, result);
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-client = { version = "0.1", features = ["all-transports"] }
```

Available features:
- `default` = `["http", "sse"]` - HTTP and SSE transport
- `http` - HTTP transport support
- `sse` - Server-Sent Events transport
- `websocket` - WebSocket transport support
- `stdio` - Standard I/O transport for executable servers
- `all-transports` - Enable all transport types

## Error Types

The client provides comprehensive error handling:

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
    McpClientError::ServerError { code, message } => {
        // Server returned an error
        eprintln!("Server error {}: {}", code, message);
    }
}
```

## Examples

### Complete Application

```rust
use turul_mcp_client::{McpClient, McpClientBuilder};
use turul_mcp_client::transport::HttpTransport;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create transport
    let transport = HttpTransport::new("http://localhost:8080/mcp")?;
    
    // Build client with retry configuration
    let client = McpClientBuilder::new()
        .with_transport(transport)
        .with_client_info("MCP Demo Client", "1.0.0")
        .with_retry_attempts(3)
        .enable_streaming()
        .build();
    
    // Connect
    info!("Connecting to MCP server...");
    client.connect().await?;
    info!("Connected successfully!");
    
    // Discover capabilities
    let tools = client.list_tools().await?;
    info!("Server provides {} tools", tools.len());
    
    for tool in &tools {
        info!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
    }
    
    // Interactive tool usage
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
    
    // Graceful cleanup
    info!("Disconnecting...");
    client.disconnect().await?;
    
    Ok(())
}
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.