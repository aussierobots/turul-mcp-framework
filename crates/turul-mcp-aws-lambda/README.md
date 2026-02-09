# turul-mcp-aws-lambda

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-aws-lambda.svg)](https://crates.io/crates/turul-mcp-aws-lambda)
[![Documentation](https://docs.rs/turul-mcp-aws-lambda/badge.svg)](https://docs.rs/turul-mcp-aws-lambda)

AWS Lambda integration for the turul-mcp-framework, enabling serverless deployment of MCP servers with full protocol compliance.

## Overview

`turul-mcp-aws-lambda` provides seamless integration between the turul-mcp-framework and AWS Lambda runtime, enabling serverless MCP servers with proper session management, CORS handling, and SSE streaming support.

## Features

- ✅ **Zero-Cold-Start Architecture** - Optimized Lambda integration
- ✅ **MCP 2025-11-25 Compliance** - Full protocol support with SSE (snapshots or streaming)
- ✅ **DynamoDB Session Storage** - Persistent session management across invocations
- ✅ **CORS Support** - Automatic CORS header injection for browser clients
- ✅ **Type Conversion Layer** - Clean `lambda_http` ↔ `hyper` conversion
- ⚠️ **SSE Support** - Snapshots via `handle()` or real streaming via `handle_streaming()`
- ✅ **Builder Pattern** - Familiar API matching `McpServer::builder()`
- ✅ **Truthful Capabilities** - Framework advertises accurate server capabilities

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-aws-lambda = "0.2.0"
turul-mcp-derive = "0.2.0"
lambda_http = "0.17"
tokio = { version = "1.0", features = ["macros"] }
```

### Basic Lambda MCP Server (Snapshot-based SSE)

```rust
use lambda_http::{run, service_fn};
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message")]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing with RUST_LOG environment variable
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    // Create Lambda MCP server with echo tool
    let server = LambdaMcpServerBuilder::new()
        .name("echo-lambda-server")
        .version("1.0.0")
        .tool(EchoTool::default())  // Add our echo tool
        .sse(true)                  // Enable SSE (snapshot-based)
        .cors_allow_all_origins()   // Allow CORS for browser clients
        .build()
        .await?;

    // Create handler for Lambda runtime
    let handler = server.handler().await?;

    // Run with standard Lambda runtime (snapshot-based SSE)
    run(service_fn(move |req| {
        let handler = handler.clone();
        async move {
            handler.handle(req).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    })).await
}
```

### Real-time Streaming Lambda MCP Server

For real-time SSE streaming, enable the `streaming` feature and use `handle_streaming()`:

```toml
[dependencies]
turul-mcp-aws-lambda = { version = "0.2", features = ["streaming"] }
```

```rust
use lambda_http::{run_with_streaming_response, service_fn};
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
// ... same tool definition ...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ... same server setup ...

    // Create handler for real-time streaming
    let handler = server.handler().await?;

    // Run with Lambda streaming response support for real-time SSE
    run_with_streaming_response(service_fn(move |req| {
        let handler = handler.clone();
        async move {
            handler.handle_streaming(req).await
        }
    })).await
}
```

## Architecture

### Framework Integration

The crate bridges AWS Lambda's HTTP execution model with the turul-mcp-framework:

```
┌─────────────────────────┐
│    AWS Lambda Runtime   │
├─────────────────────────┤
│  turul-mcp-aws-lambda   │  ← This crate
│  ├─ Type Conversion     │  ← lambda_http ↔ hyper
│  ├─ CORS Integration    │  ← Automatic header injection  
│  ├─ SSE Adaptation      │  ← Lambda streaming responses
│  └─ Session Management  │  ← DynamoDB persistence
├─────────────────────────┤
│   turul-mcp-server      │  ← Core framework
└─────────────────────────┘
```

### Three-Layer Discovery

Through lambda development, we discovered the framework's 3-layer architecture:

- **Layer 1**: `McpServer` - High-level builder and handler management
- **Layer 2**: `HttpMcpServer` - TCP server (incompatible with Lambda)
- **Layer 3**: `SessionMcpHandler` - Request handler (what Lambda needs)

This crate skips Layer 2 and provides clean integration to Layer 3.

## DynamoDB Session Storage

### Automatic Table Creation

```rust
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::DynamoDbSessionStorage;
use std::sync::Arc;

let storage = Arc::new(DynamoDbSessionStorage::new().await?);

let server = LambdaMcpServerBuilder::new()
    .name("my-lambda-server")
    .storage(storage)  // Persistent session management
    .tool(/* your tools */)
    .build()
    .await?;
```

### Session Persistence

Sessions automatically persist across Lambda invocations:

```rust
#[derive(McpTool, Clone, Default)]
#[tool(name = "counter", description = "Session-persistent counter")]
struct CounterTool;

impl CounterTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<i32> {
        if let Some(session) = session {
            let count: i32 = session.get_typed_state("count").await.unwrap_or(0);
            let new_count = count + 1;
            session.set_typed_state("count", new_count).await?;
            Ok(new_count)
        } else {
            Ok(0)
        }
    }
}
```

## CORS Configuration

### Automatic CORS for Browser Clients

```rust
let server = LambdaMcpServerBuilder::new()
    .cors_allow_all_origins()  // Enable CORS for all origins
    .build()
    .await?;
```

### Custom CORS Configuration

```rust
use turul_mcp_aws_lambda::{LambdaMcpServerBuilder, CorsConfig};

let mut cors = CorsConfig::for_origins(vec!["https://myapp.com".to_string()]);
cors.allow_credentials = true;

let server = LambdaMcpServerBuilder::new()
    .cors(cors)
    .build()
    .await?;
```

## SSE Streaming in Lambda

### Real-time Notifications

Lambda streaming responses enable real-time SSE notifications:

```rust
#[derive(McpTool, Clone, Default)]
#[tool(name = "long_task", description = "Long-running task with progress")]
struct LongTaskTool;

impl LongTaskTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            for i in 1..=5 {
                // Send progress notification via SSE
                session.notify_progress("long-task", i).await;
                
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        
        Ok("Task completed".to_string())
    }
}
```

## Deployment

### Local Testing with cargo-lambda

```bash
# Install cargo-lambda
cargo install cargo-lambda

# Run locally for testing
RUST_LOG=debug cargo lambda watch --package my-lambda-server

# Test with MCP Inspector
# Connect to: http://localhost:9000/lambda-url/my-lambda-server
```

### Deploy to AWS Lambda

```bash
# Build for Lambda
cargo lambda build --release --package my-lambda-server

# Deploy to AWS
cargo lambda deploy --package my-lambda-server
```

### Environment Configuration

```bash
# Required environment variables
export AWS_REGION=us-east-1
export MCP_SESSION_TABLE=mcp-sessions  # DynamoDB table name
export LOG_LEVEL=info
```

## Examples

### Complete AWS Integration Server

See [`examples/lambda-mcp-server`](../../examples/lambda-mcp-server) for a production-ready example with:

- DynamoDB query tools
- SNS publishing
- SQS message sending  
- CloudWatch metrics
- Full session persistence
- SSE streaming

### Builder Pattern Examples

```rust
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_builders::ToolBuilder;

// Runtime tool creation
let dynamic_tool = ToolBuilder::new("calculate")
    .description("Dynamic calculation tool")
    .number_param("x", "First number")
    .number_param("y", "Second number")
    .execute(|args| async move {
        let x = args["x"].as_f64().unwrap();
        let y = args["y"].as_f64().unwrap();
        Ok(serde_json::json!({"result": x * y}))
    })
    .build()?;

let server = LambdaMcpServerBuilder::new()
    .tool(dynamic_tool)
    .build()
    .await?;
```

## Testing

### Unit Tests

The crate includes comprehensive test coverage:

```bash
# Run all tests
cargo test --package turul-mcp-aws-lambda

# Test specific modules
cargo test --package turul-mcp-aws-lambda cors
cargo test --package turul-mcp-aws-lambda streaming
```

### Integration Testing

```bash
# Test with local Lambda runtime
cargo lambda watch &
curl -X POST http://localhost:9000/lambda-url/test \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-11-25" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

## Performance Optimization

### Cold Start Optimization

```rust
// Cache expensive operations at module level
static SHARED_STORAGE: tokio::sync::OnceCell<Arc<DynamoDbSessionStorage>> = tokio::sync::OnceCell::const_new();

async fn get_cached_storage() -> Arc<DynamoDbSessionStorage> {
    SHARED_STORAGE.get_or_init(|| async {
        Arc::new(DynamoDbSessionStorage::new().await.unwrap())
    }).await.clone()
}
```

### Memory Management

Lambda functions benefit from efficient memory usage:

```rust
let server = LambdaMcpServerBuilder::new()
    .tool(MyTool::default())  // Use Default for zero-sized types
    .build()
    .await?;
```

## Feature Flags

```toml
[dependencies]
turul-mcp-aws-lambda = { version = "0.2", features = ["cors", "sse", "dynamodb"] }
```

- `default` - Includes `cors` and `sse`
- `cors` - CORS header injection for Lambda responses
- `sse` - Server-Sent Events stream adaptation  
- `dynamodb` - DynamoDB session storage backend

## Limitations

### Lambda-Specific Considerations

- **Request Timeout**: Lambda has 15-minute maximum execution time
- **Payload Size**: 6MB maximum payload size for synchronous invocations
- **Concurrent Executions**: Subject to AWS Lambda concurrency limits
- **Cold Starts**: First invocation may have higher latency

### SSE Streaming Notes

- Lambda streaming responses have size and time limits
- Long-running SSE connections may be terminated by Lambda
- Consider API Gateway + SSE patterns for persistent-style client updates

## Error Handling

### Lambda Error Patterns

```rust
// Proper error handling for Lambda
handler.handle(req).await
    .map_err(|e| {
        tracing::error!("Lambda MCP handler error: {}", e);
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })
```

## Server Capabilities

### Truthful Capability Reporting

The framework automatically sets server capabilities based on registered components:

```rust
let server = LambdaMcpServerBuilder::new()
    .tool(calculator)
    .resource(user_resource)
    .build()
    .await?;

// Framework automatically advertises:
// - tools.listChanged = false (static tool list)
// - resources.subscribe = false (no subscriptions)
// - resources.listChanged = false (static resource list)
// - No prompts capability (none registered)
```

This ensures clients receive accurate information about server capabilities.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
