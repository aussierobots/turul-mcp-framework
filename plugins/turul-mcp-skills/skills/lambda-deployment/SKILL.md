---
name: lambda-deployment
description: >
  This skill should be used when the user asks about "lambda",
  "LambdaMcpServerBuilder", "Lambda deployment", "lambda MCP server",
  "AWS Lambda MCP", "LambdaMcpHandler", "lambda cold start",
  "OnceCell handler", "lambda SSE", "run_streaming",
  "run_streaming_with", "handle_streaming", "lambda CORS", "cors_allow_all_origins",
  "production_config", "development_config", "lambda-deployment",
  "lambda snapshot", "lambda streaming mode", or "LambdaMcpServer".
  Covers deploying MCP servers on AWS Lambda using the Turul MCP
  Framework (Rust): builder, cold-start caching, streaming vs snapshot,
  DynamoDB storage, CORS, middleware, tasks, and logging.
---

# Lambda Deployment — Turul MCP Framework

Deploy MCP servers on AWS Lambda using `LambdaMcpServerBuilder`. The Lambda crate mirrors `McpServer::builder()` but adapts for serverless: cold-start caching, DynamoDB session persistence, CORS for browser clients, and optional real-time SSE streaming.

## When to Use Lambda

```
Where does your MCP server run?
├─ Single instance, always-on ──────→ McpServer::builder()  (HTTP server)
└─ AWS Lambda / serverless ─────────→ LambdaMcpServerBuilder::new()  (this skill)
```

**Use Lambda when**: pay-per-request pricing, auto-scaling, no infrastructure management.
**Avoid Lambda when**: persistent WebSocket connections, sub-50ms latency requirements.

## Minimal Lambda Server

The smallest working Lambda MCP server:

```rust
// turul-mcp-server v0.3
use lambda_http::{Body, Error, Request, run, service_fn};
use std::sync::Arc;
use tokio::sync::OnceCell;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_session_storage::InMemorySessionStorage;

static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();

async fn create_handler() -> Result<turul_mcp_aws_lambda::LambdaMcpHandler, Error> {
    let server = LambdaMcpServerBuilder::new()
        .name("my-lambda-server")
        .version("1.0.0")
        .tool(MyTool::default())
        .storage(Arc::new(InMemorySessionStorage::new()))
        .sse(false)  // Explicitly disable SSE (on by default!)
        .cors_allow_all_origins()
        .build()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    server.handler().await.map_err(|e| Error::from(e.to_string()))
}

async fn lambda_handler(req: Request) -> Result<lambda_http::Response<Body>, Error> {
    let handler = HANDLER
        .get_or_try_init(|| async { create_handler().await })
        .await?;
    handler.handle(req).await.map_err(|e| Error::from(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(lambda_handler)).await
}
```

**See:** `examples/minimal-lambda-server.rs` for the full annotated version.

## Cold-Start Caching

```rust
static HANDLER: OnceCell<turul_mcp_aws_lambda::LambdaMcpHandler> = OnceCell::const_new();
```

Lambda reuses container instances across invocations. `OnceCell` ensures the handler (DynamoDB connections, tool registrations) is created exactly **once** per cold start, then reused for all warm invocations. No global mutable state — `OnceCell` is safe for concurrent access.

**Pattern**: `get_or_try_init()` in the request handler, or pre-initialize in `main()`:

```rust
// Option A: Lazy init on first request
let handler = HANDLER.get_or_try_init(|| async { create_handler().await }).await?;

// Option B: Eager init in main() (faster first request, slower cold start)
HANDLER.get_or_try_init(|| async { create_handler().await }).await?;
let handler = HANDLER.get().unwrap().clone();
turul_mcp_aws_lambda::run_streaming(handler).await
```

## Streaming vs Snapshot Mode

SSE is **enabled by default** (`sse` is a default feature). All 4 handler/runtime combinations work — the difference is snapshot vs real-time behavior.

```
Need real-time SSE streaming?
├─ Yes → .sse(true) + run_streaming(handler) or run_streaming_with(dispatch)  ← REAL-TIME
└─ No  → .sse(false) + handle() + run()                                       ← SNAPSHOT
```

| | Snapshot (`.sse(false)`) | Real-time (`.sse(true)` + streaming) |
|---|---|---|
| **Runtime** | `run(service_fn(...))` | `run_streaming(handler)` or `run_streaming_with(dispatch)` |
| **Handler** | `handle()` | `handle_streaming()` (called internally by `run_streaming`) |
| **GET /mcp** | 405 Method Not Allowed | Real-time SSE event stream |
| **Response body** | `LambdaBody` (buffered) | `UnsyncBoxBody<Bytes, hyper::Error>` (streaming) |
| **Cargo feature** | default features sufficient | `streaming` feature required |
| **Lambda cost** | Standard pricing | Higher (streaming response duration) |
| **Completion invocations** | Handled by `lambda_http` | Handled gracefully (no ERROR logs) |

**Two streaming entry points (v0.3+):**
- `run_streaming(handler)` — pass a `LambdaMcpHandler` directly (standard path; handles `.well-known` and other registered routes via the built-in route registry)
- `run_streaming_with(|req| async { ... })` — custom dispatch closure for pre-dispatch logic that isn't route-based (e.g., request logging, custom health checks)

Both handle API Gateway streaming completion invocations gracefully — no ERROR logs or Lambda Error metrics from completion payloads.

**Important nuances:**
- `.sse(true)` with `handle()` works but returns SSE snapshots, not real-time streams
- `handle_streaming()` with `.sse(false)` works but GET /mcp returns 405 (SSE endpoints disabled)
- For **real-time SSE**, you need: `.sse(true)` + `run_streaming()` or `run_streaming_with()` + `streaming` Cargo feature

**See:** `references/streaming-modes-guide.md` for the full streaming deep-dive.

## DynamoDB Session Storage

Lambda invocations are stateless — InMemory storage loses sessions when containers recycle. Use DynamoDB for production:

```rust
// Default table name "mcp-sessions" (hardcoded in DynamoDbConfig::default())
use turul_mcp_session_storage::DynamoDbSessionStorage;
let storage = Arc::new(DynamoDbSessionStorage::new().await?);

// Custom table name
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};
let storage = Arc::new(
    DynamoDbSessionStorage::with_config(DynamoDbConfig {
        table_name: "my-custom-table".into(),
        ..Default::default()
    }).await?
);
```

Wire into the builder with `.storage(storage)`.

**Note:** `DynamoDbSessionStorage::new()` does **not** read a `MCP_SESSION_TABLE` env var — table name is hardcoded to `"mcp-sessions"` in the default config. Use `with_config()` to customize. Default is `verify_tables: false` — tables are assumed to exist (managed by CloudFormation/Terraform). For first-time setup, use `verify_tables: true, create_tables: true`.

## CORS

Browser-based MCP clients need CORS headers. The `cors` feature is on by default.

```rust
// Development — allow all origins
.cors_allow_all_origins()

// Production — specific origins
.cors_allow_origins(vec!["https://app.example.com".into()])

// Production — from environment variables
.cors_from_env()    // Reads MCP_CORS_ORIGINS, MCP_CORS_CREDENTIALS, MCP_CORS_MAX_AGE

// Disable CORS headers entirely
.cors_disabled()
```

## Adding Middleware

Same `McpMiddleware` trait as HTTP servers. Wrap in `Arc`:

```rust
// turul-mcp-server v0.3
use std::sync::Arc;

let server = LambdaMcpServerBuilder::new()
    .name("my-server")
    .middleware(Arc::new(AuthMiddleware))
    .middleware(Arc::new(RateLimitMiddleware))
    .build()
    .await?;
```

Execution order: forward for `before_dispatch`, reverse for `after_dispatch` — same as `McpServer::builder()`.

**See:** the `middleware-patterns` skill for `McpMiddleware` trait details, error variants, and session injection.

## Adding Task Support

Long-running tools need durable task storage. DynamoDB is recommended for Lambda:

```rust
// turul-mcp-server v0.3
use turul_mcp_task_storage::DynamoDbTaskStorage;

let task_storage = Arc::new(DynamoDbTaskStorage::new().await?);

let server = LambdaMcpServerBuilder::new()
    .name("task-server")
    .with_task_storage(task_storage)
    .tool(MySlowTool::default())  // Must have task_support = "optional" or "required"
    .build()
    .await?;
```

Custom table name: `DynamoDbTaskStorage::with_config(DynamoDbTaskConfig { table_name: "my-tasks".into(), ..Default::default() }).await?`

On cold start, the handler automatically recovers stuck tasks (default timeout: 5 minutes). Configure with `.task_recovery_timeout_ms(600_000)`.

**See:** the `task-patterns` skill for task state machine, `task_support` attribute, and cancellation details.

## Environment Variables

| Variable | Default | Purpose | Read by |
|---|---|---|---|
| `AWS_REGION` | `us-east-1` | AWS region | `DynamoDbConfig::default()` |
| `MCP_SESSION_EVENT_TABLE` | `{table_name}-events` | DynamoDB event table | `DynamoDbSessionStorage` |
| `MCP_CORS_ORIGINS` | (none) | CORS allowed origins | `.cors_from_env()` |
| `MCP_CORS_CREDENTIALS` | (none) | CORS credentials | `.cors_from_env()` |
| `MCP_CORS_MAX_AGE` | (none) | CORS max-age | `.cors_from_env()` |
| `LOG_LEVEL` | `INFO` | tracing log level | tracing subscriber |

**Note:** DynamoDB session table name is NOT configurable via env var — it defaults to `"mcp-sessions"` in `DynamoDbConfig::default()`. Use `with_config()` to customize.

## Lambda Logging

CloudWatch-optimized logging setup:

```rust
tracing_subscriber::fmt()
    .with_max_level(log_level.parse().unwrap_or(tracing::Level::INFO))
    .with_target(false)   // CloudWatch doesn't need target
    .without_time()       // CloudWatch adds timestamps
    .json()               // Structured JSON for CloudWatch Logs Insights
    .init();
```

**Tip:** Check `AWS_EXECUTION_ENV` to switch between JSON (Lambda) and human-readable (local dev) logging.

## API Gateway Authorizer Integration

API Gateway authorizers populate `x-authorizer-*` headers automatically. Middleware reads them via `ctx.metadata()`:

```rust
let user_id = ctx.metadata()
    .get("x-authorizer-principalid")
    .and_then(|v| v.as_str());
```

The Lambda adapter converts camelCase authorizer fields to snake_case headers (`userId` → `x-authorizer-user_id`). Both V1 (REST API) and V2 (HTTP API) formats are supported.

**See:** the `middleware-patterns` skill (Pattern 4: Lambda Auth) for the full `LambdaAuthMiddleware` example.

## Convenience Presets

```rust
// Production: DynamoDB sessions + env-based CORS
// Requires features: dynamodb + cors
let server = LambdaMcpServerBuilder::new()
    .name("prod-server")
    .version("1.0.0")
    .production_config().await?   // DynamoDbSessionStorage::new() + cors_from_env()
    .tool(MyTool::default())
    .build()
    .await?;

// Development: InMemory sessions + allow-all CORS
// Requires feature: cors
let server = LambdaMcpServerBuilder::new()
    .name("dev-server")
    .version("1.0.0")
    .development_config()          // InMemorySessionStorage + cors_allow_all_origins()
    .tool(MyTool::default())
    .build()
    .await?;
```

## Common Mistakes

1. **Using `InMemorySessionStorage` in production Lambda** — Sessions are lost when containers recycle. Use `DynamoDbSessionStorage` for persistence across invocations.

2. **Using `handle()` with `.sse(true)` expecting real-time streaming** — `handle()` returns SSE snapshots, not real-time streams. For real-time SSE, use `run_streaming(handler)` or `run_streaming_with(dispatch)` + the `streaming` Cargo feature.

3. **Expecting `.sse(false)` as default behavior** — SSE is enabled by default when the `sse` feature is active (which it is in default features). Explicitly call `.sse(false)` if you don't want SSE.

4. **Forgetting `OnceCell` caching** — Without `OnceCell`, the handler is recreated on every invocation. This means new DynamoDB connections, tool registrations, and session managers every cold start.

5. **Assuming `DynamoDbSessionStorage::new()` reads a table name env var** — It doesn't. The table name is hardcoded to `"mcp-sessions"` in `DynamoDbConfig::default()`. Use `DynamoDbSessionStorage::with_config()` to customize.

6. **Using `.sse(true)` without the `streaming` Cargo feature** — SSE works but only as snapshots. The `streaming` feature is required for `run_streaming()` / `run_streaming_with()` to provide real-time SSE.

## Beyond This Skill

**Middleware details?** → See the `middleware-patterns` skill for `McpMiddleware` trait, error variants, and session injection.

**Task state machine?** → See the `task-patterns` skill for task lifecycle, `task_support` declaration, and cancellation.

**Storage backend config?** → See the `storage-backend-matrix` reference for DynamoDB/SQLite/PostgreSQL feature flags and Cargo.toml patterns.

**Session storage architecture?** → See the `session-storage-backends` skill for the SessionStorage trait, backend decision tree, event management, and error types.

**Error handling in tools?** → See the `error-handling-patterns` skill for `McpError` variants and decision tree.

**Client-side workflows?** → See the `mcp-client-patterns` skill for transport selection and tool invocation.

**Builder API reference?** → See `references/lambda-builder-reference.md` for the full `LambdaMcpServerBuilder` API.
