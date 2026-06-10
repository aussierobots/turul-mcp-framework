# turul-mcp-aws-lambda

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-aws-lambda.svg)](https://crates.io/crates/turul-mcp-aws-lambda)
[![Documentation](https://docs.rs/turul-mcp-aws-lambda/badge.svg)](https://docs.rs/turul-mcp-aws-lambda)

AWS Lambda integration for the turul-mcp-framework, enabling serverless deployment of MCP servers with full protocol compliance.

## Overview

`turul-mcp-aws-lambda` provides seamless integration between the turul-mcp-framework and AWS Lambda runtime, enabling serverless MCP servers with proper session management, CORS handling, and SSE streaming support.

## Features

- ✅ **Zero-Cold-Start Architecture** - Optimized Lambda integration
- ✅ **MCP 2026-07-28 Default** - Stateless core (`server/discover`, per-request `_meta`, no `Mcp-Session-Id`) with SSE (snapshots or streaming); 2025-11-25 sessionful handshake is opt-in
- ✅ **DynamoDB Session Storage** - Persistent session management across invocations
- ✅ **CORS Support** - Automatic CORS header injection for browser clients
- ✅ **Type Conversion Layer** - Clean `lambda_http` ↔ `hyper` conversion
- ⚠️ **SSE Support** - Snapshots via `handle()` or real streaming via `handle_streaming()`
- ✅ **Builder Pattern** - Familiar API matching `McpServer::builder()`
- ✅ **Truthful Capabilities** - Framework advertises accurate server capabilities
- ✅ **MCP Tasks Support** - Task-augmented `tools/call` with durable storage persistence

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-aws-lambda = "0.4"
turul-mcp-derive = "0.4"
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

    // Build the handler eagerly in main() — see ADR-024.
    let handler = server.handler().await?;

    // Run with standard Lambda runtime (snapshot-based SSE).
    run(service_fn(move |req| dispatch(handler.clone(), req))).await
}

async fn dispatch(
    handler: turul_mcp_aws_lambda::LambdaMcpHandler,
    req: lambda_http::Request,
) -> Result<lambda_http::Response<lambda_http::Body>, Box<dyn std::error::Error + Send + Sync>> {
    handler.handle(req).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}
```

### Real-time Streaming Lambda MCP Server

For real-time SSE streaming, enable the `streaming` feature and use `run_streaming()`:

```toml
[dependencies]
turul-mcp-aws-lambda = { version = "0.4", features = ["streaming"] }
```

```rust
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
// ... same tool definition ...

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    // ... same server setup ...

    let handler = server.handler().await?;

    // run_streaming() handles API Gateway completion invocations gracefully —
    // no ERROR logs or Lambda Error metrics from completion payloads.
    turul_mcp_aws_lambda::run_streaming(handler).await
}
```

#### Custom Dispatch with `run_streaming_with()`

When you need pre-dispatch logic (e.g., `.well-known` routing) that runs before
the MCP handler, use `run_streaming_with()`. Build the handler eagerly in
`main()` and `move`-capture it into the closure — this keeps init cost in
Lambda's `Init Duration` rather than the first request's `handler_total`:

```rust
let handler = server.handler().await?;
turul_mcp_aws_lambda::run_streaming_with(move |request| {
    let handler = handler.clone();
    async move {
        if request.uri().path() == "/.well-known/oauth-authorization-server" {
            return Ok(well_known_response());
        }
        handler.handle_streaming(request).await
    }
}).await
```

> **Recommendation: build the handler eagerly in `main()`.** Construct
> `LambdaMcpServerBuilder` and call `.build().await?.handler().await?` *before*
> handing control to the Lambda runtime. Avoid `OnceCell::get_or_try_init(...)`
> inside the per-request dispatch path for latency-sensitive or fan-out
> callers — that pattern moves DDB session/server-state init, server build, and
> tool registration (~500 ms with DDB backends) inside the first invocation's
> `Duration` instead of Lambda's `Init Duration` field. Lazy request-path init
> remains valid for back-compat. See [ADR-024](../../docs/adr/024-lambda-eager-handler-init.md).

Both entry points classify raw Lambda runtime payloads three ways:
- **API Gateway events** — dispatched to your handler normally
- **Streaming completion invocations** — acknowledged silently (debug log)
- **Unrecognized payloads** — acknowledged with a warn log

This fixes the ERROR logs and CloudWatch Lambda Error metrics caused by
completion invocations when using `lambda_http::run_with_streaming_response()`
directly. It does **not** claim to resolve all Lambda streaming timeout
behavior — that remains a separate concern.

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
│  ├─ Session Management  │  ← DynamoDB persistence
│  └─ Task Runtime        │  ← MCP Tasks with durable storage
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

## MCP Tasks Support

Enable task-augmented `tools/call` for long-running operations. When a client sends `tools/call` with a `task` parameter, the server creates a task record, dispatches execution asynchronously, and returns a `CreateTaskResult` immediately.

### Task Storage Configuration

```rust
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_server::task_storage::InMemoryTaskStorage;
use std::sync::Arc;

// For production: use DynamoDB task storage
let task_storage = Arc::new(InMemoryTaskStorage::new());

let server = LambdaMcpServerBuilder::new()
    .name("my-lambda-server")
    .tool(MyTool::default())
    .with_task_storage(task_storage)       // Enables tasks/get, tasks/list, etc.
    .task_recovery_timeout_ms(300_000)     // 5 min stuck-task recovery (default)
    .build()
    .await?;
```

### How It Works

1. Client sends `tools/call` with `{ "task": {} }` parameter
2. Server creates a task record in durable storage (status: `working`)
3. Server returns `CreateTaskResult` immediately (non-blocking)
4. Tool execution runs asynchronously via the task executor
5. Client polls `tasks/get` or waits on `tasks/result` for completion

### Lambda-Specific Considerations

- **Durable storage required**: Use DynamoDB for task storage. `InMemoryTaskStorage` loses state between Lambda invocations.
- **Post-response task completion is best-effort** (operational limitation): The `tools/call` request path is non-blocking and returns `CreateTaskResult` immediately. However, the background tool work (`tokio::spawn`) may not complete before Lambda freezes the execution environment. Short-lived tools that finish within the invocation work reliably. Long-running tools MUST use an external updater (Step Functions, callback Lambda) to drive task completion via durable storage. This is a Lambda platform constraint, not a framework bug.
- **Stuck-task recovery**: On each Lambda cold start, `recover_stuck_tasks()` marks stale `Working` tasks as `Failed` (configurable via `task_recovery_timeout_ms`).
- **Cross-invocation cancellation is best-effort**: `tasks/cancel` updates storage status, but cannot signal a frozen Lambda invocation. Work may complete after cancellation.
- **`tasks/result` polling**: When the executor doesn't track a task (different invocation), the handler falls back to 500ms storage polling with a 5-minute timeout.
- **Cost optimization**: Lambda billing is request + duration based; reducing invocation duration usually reduces cost. Non-blocking task dispatch returns `CreateTaskResult` fast and frees the invocation rather than holding it open waiting for tool completion.

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

### Deployment notes — REST API Gateway

The crate registers the Lambda binary as **streaming-only** via
`lambda_runtime::run` with a `StreamResponse` return type. That registration is
a per-binary contract with the AWS Runtime API: a single binary cannot serve
both streaming and buffered framings. Streaming responses are not
buffered-Invoke compatible, so REST API Gateway integrations need a few
deployment-side rules:

- **OPTIONS preflight**: REST `AWS_PROXY` integrations talking to a
  streaming-registered Lambda cannot reliably parse the streaming
  response envelope. Use an **API Gateway MOCK integration** for the
  `OPTIONS` method on browser-facing routes (e.g. `/mcp`). This is the
  AWS-recommended pattern for streaming Lambdas and matches what the
  console's *Enable CORS* button auto-generates.
- **API-Gateway-emitted 4xx/5xx**: Errors produced by API Gateway itself
  (authorizer rejection, throttling, internal gateway failures) never
  reach the Lambda, so no framework-side CORS injection can apply. Add
  `AWS::ApiGateway::GatewayResponse` resources for at least
  `UNAUTHORIZED`, `ACCESS_DENIED`, `DEFAULT_4XX`, and `DEFAULT_5XX` with
  `Access-Control-Allow-Origin` (and friends) set via the
  `gatewayresponse.header.*` parameters.
- **Browser OAuth (`WWW-Authenticate`)**: Browsers can only read
  non-safelisted response headers when listed in
  `Access-Control-Expose-Headers`. RFC 9728 OAuth discovery requires
  clients to parse `WWW-Authenticate` on `401` responses, so it must be
  exposed. The default `CorsConfig` includes it; if you build a custom
  `CorsConfig` and want browser OAuth to work, retain
  `WWW-Authenticate` in `expose_headers`.
- **`.well-known` / RFC 9728 metadata routes**: any route handler
  registered via `LambdaMcpServerBuilder::route(...)` is reached
  *before* MCP dispatch in both buffered and streaming paths. Both
  paths apply the configured `CorsConfig` to the route response before
  returning. If you bypass this — e.g. by handling the route inside a
  `run_streaming_with` dispatch closure that short-circuits before
  calling `handler.handle_streaming()` — you must call the re-exported
  helpers yourself:

  ```rust
  use turul_mcp_aws_lambda::{inject_cors_headers, create_preflight_response};
  // ... inside your dispatch closure, before returning:
  if let Some(cors) = &cors_config {
      inject_cors_headers(&mut response, cors, request_origin.as_deref())?;
  }
  ```

- **Per-route method contracts**: `inject_cors_headers` applies
  `CorsConfig.allowed_methods` as configured. It does **not** infer
  that a `.well-known` route only allows `GET, OPTIONS` while `/mcp`
  allows `GET, POST, DELETE, OPTIONS`. If you need per-route method
  advertising, either build separate `CorsConfig` instances per route
  or accept the unified list.

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
# Test with local Lambda runtime — 2025-11-25 opt-in lane shown
# (the default 2026-07-28 build is stateless: POST server/discover, no initialize, no Mcp-Session-Id)
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
turul-mcp-aws-lambda = { version = "0.4", features = ["cors", "sse", "dynamodb"] }
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
