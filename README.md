# Turul MCP Framework

Build [Model Context Protocol (MCP)](https://modelcontextprotocol.io) servers and clients in Rust — from a five-line tool server to serverless AWS Lambda deployments.

Turul gives you the full MCP surface (tools, resources, prompts, completion, notifications, extensions) behind a zero-configuration builder API: annotate a function or derive on a struct, add it to the builder, run. The framework generates the schemas, wires the transport, and keeps you spec-compliant.

The default build targets **MCP 2026-07-28**, the current specification. The previous spec, **2025-11-25**, stays fully supported as an opt-in build (`--no-default-features --features protocol-2025-11-25`). Servers are single-spec per build; the client speaks both and negotiates per connection.

> **Pre-1.0 (0.4.x):** production-shaped — comprehensive test coverage, zero-warning gates — but public APIs may still change before 1.0.0.

## ✨ Key Highlights

- **🏗️ 17 Framework Crates**: Complete MCP ecosystem with core framework, client library, task storage, serverless support, and opt-in protocol extensions (Tasks, Apps)
- **📚 Comprehensive Examples**: Real-world business applications and framework demonstrations (most build on the 2026-07-28 default lane; a 2025-11-25 regression set is pinned to the opt-in, and client-using examples are built by explicit CI steps — see EXAMPLES.md)
- **🧪 Framework-Native Test Suite**: Core framework tests, SessionContext integration tests, and framework-native integration tests
- **⚡ Multiple Development Patterns**: Derive macros, function attributes, declarative macros, and manual implementation
- **🌐 Transport Flexibility**: Streamable HTTP via StreamableHttpHandler with SSE streaming (stdio planned)
- **☁️ Serverless Support**: AWS Lambda integration with streaming responses and SQS event processing
- **🔧 Development Features**: Session management, real-time notifications, performance monitoring, and UUID v7 support

## 🚀 Quick Start

### 1. Function Macros (Simplest - Recommended)

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)  // Function macro wraps as {"result": 8.0} (override with output_field)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add)  // Use function name directly
        .bind_address("127.0.0.1:8641".parse()?)  // Default port; customize as needed
        .build()?;

    server.run().await
}
```

> **Task support:**
> - **2026-07-28** — tasks are the `io.modelcontextprotocol/tasks` extension (SEP-2663). Enable the `ext-tasks` feature, register the store with `.with_ext_tasks(store)`, and mark electable tools with `.ext_task_tool(tool)`. See `examples/ext-tasks-server`.
> - **2025-11-25 opt-in** — tasks are a core capability: add `task_support = "optional"` (`"optional"` / `"required"` / `"forbidden"`) to enable the "Run as Task" button in MCP Inspector; requires `.with_task_storage()` on the builder.

### 2. Derive Macros (Struct-Based)

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone)]
#[tool(name = "calculator", description = "Mathematical operations")]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
    #[param(description = "Operation (+, -, *, /)")]
    operation: String,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        match self.operation.as_str() {
            "+" => Ok(self.a + self.b),
            "-" => Ok(self.a - self.b),
            "*" => Ok(self.a * self.b),
            "/" => {
                if self.b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(self.a / self.b)
                }
            },
            _ => Err("Invalid operation".into()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0, operation: "+".to_string() })
        .bind_address("127.0.0.1:8642".parse()?)  // Different port to avoid conflicts
        .build()?;

    server.run().await
}
```

### 3. Resources with resource_fn()

Create resources that provide data and files using the `.resource_fn()` method:

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
        "version": "1.0.0"
    });

    Ok(vec![ResourceContent::blob(
        "file:///config.json",
        serde_json::to_string_pretty(&config).unwrap(),
        "application/json".to_string()
    )])
}

// Template resource with parameter extraction
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
        serde_json::to_string_pretty(&profile).unwrap(),
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
        .bind_address("127.0.0.1:8643".parse()?)  // Different port to avoid conflicts
        .build()?;

    server.run().await
}
```

The framework automatically:
- Detects URI templates (`{user_id}` patterns)
- Extracts template variables from requests
- Maps them to function parameters
- Registers appropriate resource handlers

## The MCP 2026-07-28 specification

The default build targets MCP 2026-07-28, the current specification — a **stateless
rewrite** of the protocol with several core methods removed. New to MCP? You can skim
this section and come back later. Migrating from 2025-11-25? Read it first.

### What's new (and implemented here)

- **Stateless core** — `server/discover` advertises versions/capabilities/identity;
  every request carries `protocolVersion` + `clientInfo` + `clientCapabilities` in
  `_meta` under `io.modelcontextprotocol/*`. Any server instance serves any request.
- **MRTR (Multi-Round-Trip Requests, SEP-2322)** — a tool/resource/prompt returns an
  `InputRequiredResult` to ask for elicitation/sampling/roots input; the client
  answers and retries the original call. This replaces all server-initiated requests.
  → `examples/mrtr-elicitation-server`
- **`subscriptions/listen`** — the ack-first, filtered, long-lived POST SSE stream that
  replaces 2025's GET-SSE resumability and the `resources/subscribe` RPC.
- **Tasks extension** (`io.modelcontextprotocol/tasks`, SEP-2663) — durable poll handles
  via `turul-mcp-ext-tasks` (opt-in `ext-tasks` feature): `.with_ext_tasks()` +
  `.ext_task_tool()` server-side, `call_tool_or_task` / `task_*` client-side.
  → `examples/ext-tasks-server`
- **MCP Apps extension** (`io.modelcontextprotocol/ui`, SEP-1865) — MCP-side bindings
  (`turul-mcp-ext-apps`).
- **SEP-2243 request-metadata headers** — `Mcp-Method` / `Mcp-Name` / `Mcp-Param-*` let
  infrastructure route on method/tool/arguments without parsing JSON bodies.
  → `examples/header-bound-tools-server`
- **Origin validation / DNS-rebinding protection** — on by default (ADR-031).
  → `examples/origin-policy-server`
- **JSON Schema 2020-12**, **caching headers** (`ttlMs` / `cacheScope` on list/read
  results), and OAuth 2.1 Resource Server hardening (RFC 9207/9728).

### What changed from 2025-11-25

- **No handshake** — `initialize` → `notifications/initialized` → `Mcp-Session-Id` is
  gone; capabilities ride per-request `_meta` instead.
- **Error codes** — resource-not-found moved from `-32002` to `-32602`; new
  `-32020` (header mismatch), `-32021` (missing required client capability),
  `-32022` (unsupported protocol version).
- **Per-request logging** — log level is set per-request via `io.modelcontextprotocol/logLevel`
  in `_meta`; servers must not emit `notifications/message` for requests that didn't opt in.
- **Deprecations (SEP-2577)** — Roots, Sampling, and Logging are deprecated (earliest
  removal 2027-07-28). Still implemented; on 2026 the server-initiated forms ride MRTR.

### What's removed from the core (2026 default → these 404 / 405)

| Removed | SEP | Replacement on 2026 |
|---|---|---|
| `initialize` / `notifications/initialized` | 2575 | `server/discover` + per-request `_meta` |
| Protocol sessions / `Mcp-Session-Id` | 2567 | stateless; server-minted handles as tool args |
| `ping` | 2575 | — (use `server/discover` for liveness) |
| `logging/setLevel` | 2575 | per-request `_meta.logLevel` |
| `notifications/roots/list_changed` | 2575 | — |
| GET SSE endpoint / `resources/subscribe` | 2575 | `subscriptions/listen` |
| Core `tasks/*` (incl. `tasks/list`, blocking `tasks/result`) | 2663 | the Tasks **extension** (`tasks/get` polling + `tasks/update`) |

**2025-11-25 stays fully supported** as the opt-in stateful lane
(`--no-default-features --features protocol-2025-11-25`) — the handshake, sessions,
GET SSE, and core tasks all work there. Per-requirement compliance status:
[`docs/plans/2026-07-28-spec-compliance.md`](docs/plans/2026-07-28-spec-compliance.md).

## 🚀 Running & Testing the Framework

### Quick Start - Verify Everything Works

```bash
# 1. Build the framework
cargo build --workspace

# 2. Run compliance tests
cargo test -p turul-mcp-framework-integration-tests --test compliance

# 3. Start a simple server
cargo run -p minimal-server

# 4. Test the server (in another terminal) — 2026-07-28 stateless: no handshake,
#    capabilities travel in per-request `_meta`, no Mcp-Session-Id header.
curl -X POST http://127.0.0.1:8641/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{"jsonrpc":"2.0","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}},"id":1}'
```

### Example Servers - Ready to Run

**Core Test Servers** (manifest-pinned to the **2025-11-25** opt-in lane — they are the stateful regression fixtures, so use the 2025 handshake against them, not the 2026 `server/discover` curls below):
```bash
# Tools server (test tools)
cargo run --package tools-test-server -- --port 8002

# Resource server (test resources)
cargo run --package resource-test-server -- --port 8080

# Prompts server (test prompts)
cargo run --package prompts-test-server -- --port 8081
```

**Business Application Servers:**
```bash  
# Development team resources
cargo run -p resources-server -- --port 8041

# AI development prompts  
cargo run -p prompts-server -- --port 8040

# Real-time notifications
cargo run -p notification-server

# Session management demo (2025-11-25 stateful lane — manifest-pinned)
cargo run -p stateful-server
```

### Manual MCP Compliance Verification (2026-07-28 default)

The default build is stateless: there is no `initialize` handshake and no `Mcp-Session-Id`
header. Capabilities, client info, and the protocol version travel in `_meta` on every
request under the `io.modelcontextprotocol/*` namespace. Use a **2026-default** server such
as `minimal-server` (port 8641) — the `*-test-server` fixtures above are 2025-pinned and
will not answer `server/discover`.

**Step 1: Discover the server**
```bash
cargo run -p minimal-server   # 2026 default, binds 127.0.0.1:8641
PORT=8641  # a 2026-default server's port
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{
    "jsonrpc": "2.0",
    "method": "server/discover",
    "params": {
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    },
    "id": 1
  }' | jq
```

**Step 2: Call operations (no session header)**
```bash
# Tools — capabilities ride in per-request `_meta`:
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/list' \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}},"id":2}' | jq

# Resources:
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/list' \
  -d '{"jsonrpc":"2.0","method":"resources/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}},"id":3}' | jq
```

**2025-11-25 (opt-in):** build with `--no-default-features --features protocol-2025-11-25`
and the stateful handshake applies instead — `initialize` → `notifications/initialized` →
`Mcp-Session-Id` header on every subsequent request:
```bash
curl -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | jq
```

### Comprehensive Testing Guide

For detailed testing instructions, server running guides, and compliance verification:

**📚 Testing**: lane-by-lane gates run via `./scripts/ci-gates.sh all`; the per-requirement test inventory is `docs/plans/2026-07-28-spec-compliance.md` §E2E test plan

This guide includes:
- ✅ All server running instructions with expected outputs
- ✅ Manual MCP compliance verification (2026-07-28 default; 2025-11-25 opt-in)  
- ✅ SSE event stream testing procedures
- ✅ Performance testing and troubleshooting
- ✅ CI/CD integration examples

### Quick Compliance Check Script

```bash
# Create and run compliance check
cat > quick_check.sh << 'EOF'
#!/bin/bash
PORT=${1:-8080}
echo "🧪 Testing MCP server on port $PORT"

DISCOVER_RESPONSE=$(curl -s -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{"jsonrpc":"2.0","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}},"id":1}')

if [[ $(echo $DISCOVER_RESPONSE | jq -r '.result.serverInfo.name // empty') != "" ]]; then
    echo "✅ MCP 2026-07-28 server/discover responded"
else
    echo "❌ Not compliant"
    exit 1
fi
EOF

chmod +x quick_check.sh

# Test any server
cargo run -p minimal-server &
./quick_check.sh 8641
```

## Turul MCP vs Turul RPC

This project ships two layered surfaces. Most users only need the MCP layer.

- **`turul-mcp`** (this framework) — the Model Context Protocol implementation. Tools, resources, prompts, sampling, elicitation, tasks, sessions, Streamable HTTP/SSE transport, the macro suite, storage backends.
- **[`turul-rpc`](https://github.com/aussierobots/turul-rpc)** — generic, transport-agnostic typed JSON-RPC 2.0 framework: dispatch, domain/protocol error separation, optional session context, async handler trait, batch processing, notifications. No MCP knowledge. Useful as a foundation for any non-MCP request/response service that wants the same handler-returns-domain-error contract Turul uses internally.

Turul MCP is built on top of Turul RPC. The `turul-mcp-json-rpc-server` crate, which historically carried the JSON-RPC implementation, is a thin re-export shim over `turul-rpc` since v0.3.39. **Existing 0.3.x users do not need to change anything** — `turul_mcp_json_rpc_server::*` imports continue to compile and behave identically. New code (and new tools/agents reading this README) should depend on `turul-rpc` directly. See [ADR-025](docs/adr/025-extract-turul-rpc.md).

## 🏛️ Architecture Overview

### Middleware System

The framework provides a trait-based middleware architecture for cross-cutting concerns like authentication, logging, and rate limiting:

```rust
use turul_mcp_server::prelude::*;
use std::sync::Arc;

let server = McpServer::builder()
    .middleware(Arc::new(AuthMiddleware::new()))
    .middleware(Arc::new(LoggingMiddleware))
    .middleware(Arc::new(RateLimitMiddleware::new(5, 60)))
    .build()?;
```

**Key Features:**
- ✅ Transport-agnostic (HTTP, Lambda, etc.)
- ✅ Session-aware (read/write session state)
- ✅ Error short-circuiting with semantic JSON-RPC codes
- ✅ Execution order control (FIFO before, LIFO after dispatch)

**Examples:**
- `examples/middleware-logging-server` - Request timing and tracing (HTTP)
- `examples/middleware-rate-limit-server` - Per-session rate limiting (HTTP)
- `examples/middleware-auth-server` - API key authentication (HTTP)
- `examples/middleware-auth-lambda` - API key authentication (AWS Lambda)

**Testing:**
- Test HTTP middleware: `bash scripts/test_middleware_live.sh`
- Test Lambda middleware: `cargo lambda watch --package middleware-auth-lambda`

**Documentation:**
- [ADR 012: Middleware Architecture](docs/adr/012-middleware-architecture.md) - Core middleware design
- [ADR 013: Lambda Authorizer Integration](docs/adr/013-lambda-authorizer-integration.md) - API Gateway authorizer support

#### Lambda Authorizer Integration

**Seamless API Gateway authorizer context extraction for Lambda deployments:**

```rust
// API Gateway authorizer adds context (userId, tenantId, role, etc.)
// → turul-mcp-aws-lambda adapter extracts → injects x-authorizer-* headers
// → Middleware reads headers → stores in session state
// → Tools access via session.get_typed_state("authorizer")

#[async_trait]
impl McpMiddleware for AuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Extract authorizer context from x-authorizer-* headers
        let metadata = ctx.metadata();
        let mut authorizer_context = HashMap::new();

        for (key, value) in metadata.iter() {
            if let Some(field_name) = key.strip_prefix("x-authorizer-") {
                if let Some(value_str) = value.as_str() {
                    authorizer_context.insert(field_name.to_string(), value_str.to_string());
                }
            }
        }

        if !authorizer_context.is_empty() {
            // Store for tools to access
            injection.set_state("authorizer", json!(authorizer_context));
        }

        Ok(())
    }
}
```

**Key Features:**
- ✅ Supports API Gateway V1 (REST API) and V2 (HTTP API)
- ✅ Field name sanitization (camelCase → snake_case: `userId` → `user_id`)
- ✅ Defensive programming (never fails requests)
- ✅ Transport-agnostic (appears as standard HTTP metadata)
- ✅ Session state integration

**Example:**
- `examples/middleware-auth-lambda` - Full authorizer extraction pattern (V1 nested, V1 flat, V2)
- Test events: V1 nested, V1 flat, V2 authorizer shapes (`test-events/`)

### Core Framework (17 Crates)
- **`turul-mcp-server`** - High-level server builder with session management and task runtime
- **`turul-mcp-client`** - Comprehensive client library with HTTP transport support (bilingual: 2026-07-28 + 2025-11-25)
- **`turul-http-mcp-server`** - HTTP/SSE transport with CORS and streaming
- **`turul-mcp-protocol`** - Current MCP specification alias (defaults to 2026-07-28; `protocol-2025-11-25` feature for the prior spec)
- **`turul-mcp-protocol-2026-07-28`** - MCP 2026-07-28 specification implementation (default active spec)
- **`turul-mcp-protocol-2025-11-25`** - MCP 2025-11-25 specification implementation (frozen historical snapshot; opt-in)
- **`turul-mcp-protocol-2025-06-18`** - Legacy MCP specification (frozen historical snapshot)
- **`turul-mcp-derive`** - Procedural macros for all MCP areas
- **`turul-mcp-builders`** - Runtime builder patterns for dynamic MCP components
- **`turul-mcp-ext-tasks`** - Tasks extension (`io.modelcontextprotocol/tasks`, SEP-2663) for the 2026-07-28 lane (opt-in `ext-tasks` feature)
- **`turul-mcp-ext-apps`** - MCP Apps extension (`io.modelcontextprotocol/ui`, SEP-1865) — MCP-side bindings
- **`turul-mcp-json-rpc-server`** - Frozen 0.3.x compatibility shim re-exporting [`turul-rpc`](https://github.com/aussierobots/turul-rpc); the framework crates depend on `turul-rpc` directly (see ADR-025)
- **`turul-mcp-session-storage`** - Session storage backends (SQLite, PostgreSQL, DynamoDB)
- **`turul-mcp-task-storage`** - Task storage for long-running operations (InMemory, with pluggable backends)
- **`turul-mcp-server-state-storage`** - Server-global state for dynamic tool coordination
- **`turul-mcp-aws-lambda`** - AWS Lambda integration for serverless deployment
- **`turul-mcp-oauth`** - OAuth 2.1 Resource Server support (JWT validation, Bearer middleware)

### Tasks Architecture

Tasks moved from a **core capability (2025-11-25)** to an **extension (2026-07-28)**, and the framework implements both:

- **2026-07-28** — the `io.modelcontextprotocol/tasks` extension (SEP-2663) in `turul-mcp-ext-tasks`, wired into the server behind the opt-in `ext-tasks` feature (`.with_ext_tasks(store)` + `.ext_task_tool(...)`) and the client (`call_tool_or_task`, `task_get`/`task_update`/`task_cancel`/`task_wait`). Server election, the `-32003` capability gate, mid-task input via `tasks/update`, and `notifications/tasks` over `subscriptions/listen` are all implemented. See `examples/ext-tasks-server`.
- **2025-11-25 opt-in** — the original in-tree core task runtime, gated to `#[cfg(feature = "protocol-2025-11-25")]` (protocol types, storage, runtime, handlers, tests).

Architecture decision records:

- [ADR-028: Extensions Strategy](docs/adr/028-extensions-strategy.md) — per-extension crates; how the 2026 Tasks/Apps extensions are hosted
- [ADR-015: Protocol Crate Strategy](docs/adr/015-mcp-2025-11-25-protocol-crate.md) — separate crate for 2025-11-25 spec types including core Tasks
- [ADR-016: Task Storage Architecture](docs/adr/016-task-storage-architecture.md) — `TaskStorage` trait, 4 backends, state machine, parity test suite
- [ADR-017: Task Runtime-Executor Boundary](docs/adr/017-task-runtime-executor-boundary.md) — three-layer split: storage / executor / runtime
- [ADR-018: Task Pagination Cursor Contract](docs/adr/018-task-pagination-cursor-contract.md) — deterministic cursor-based pagination across backends

### Fine-Grained Trait Architecture
**Modern composable design pattern for all MCP areas:**

```rust
use turul_mcp_builders::prelude::*;  // Framework traits + builders
use turul_mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
use turul_mcp_server::{McpTool, SessionContext};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

struct MyTool;

// Fine-grained trait composition for maximum flexibility
impl HasBaseMetadata for MyTool {
    fn name(&self) -> &str { "my_tool" }
}

impl HasDescription for MyTool {
    fn description(&self) -> Option<&str> { Some("Tool description") }
}

impl HasInputSchema for MyTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("input".to_string(), JsonSchema::string())
                ]))
        })
    }
}

impl HasIcons for MyTool {}     // No icons (default)
impl HasExecution for MyTool {} // No task support (default)

// ToolDefinition automatically implemented via blanket impl
#[async_trait]
impl McpTool for MyTool {
    async fn call(&self, _args: Value, _session: Option<SessionContext>) 
        -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![
            ToolResult::text("Tool result")
        ]))
    }
}
```

**Supported Areas:**
- **Tools** (`ToolDefinition`) - Dynamic tool execution with validation
- **Resources** (`ResourceDefinition`) - Static and dynamic content serving
- **Prompts** (`PromptDefinition`) - AI interaction template generation
- **Sampling** (`SamplingDefinition`) - AI model integration patterns
- **Completion** (`CompletionDefinition`) - Context-aware text completion
- **Logging** (`LoggerDefinition`) - Dynamic log level management
- **Roots** (`RootDefinition`) - Secure file system access boundaries
- **Elicitation** (`ElicitationDefinition`) - Structured user input collection
- **Notifications** (`NotificationDefinition`) - Real-time event broadcasting

> **2026-07-28 note:** Roots, Sampling, and Logging are deprecated in the 2026-07-28 spec
> (SEP-2577, earliest removal 2027-07-28). They remain implemented; on the 2026 default
> the server-initiated forms ride Multi Round-Trip Requests, and `notifications/message`
> requires the per-request `logLevel` opt-in.

### Comprehensive Server Builder
**All MCP areas supported with consistent builder pattern:**

```rust
let server = McpServer::builder()
    .name("comprehensive-server")
    .version("1.0.0")
    .instructions("Full-featured MCP server with all areas")
    // Tools
    .tool(WeatherTool::new())
    .tools(vec![CalculatorTool::new(), ValidationTool::new()])
    // Resources
    .resource(AppConfigResource::new())
    .resources(vec![LogsResource::new(), MetricsResource::new()])
    // Prompts
    .prompt(CodeReviewPrompt::new())
    .prompts(vec![DocumentationPrompt::new(), TestPrompt::new()])
    // Sampling
    .sampling_provider(CreativeSampling::new())
    .sampling_providers(vec![CodeSampling::new(), TechnicalSampling::new()])
    // Completion
    .completion_provider(IdeCompletion::new())
    .completion_providers(vec![SqlCompletion::new(), JsonCompletion::new()])
    // Logging
    .logger(AuditLogger::new())
    .loggers(vec![SecurityLogger::new(), PerformanceLogger::new()])
    // Roots
    .root_provider(WorkspaceRoot::new())
    .root_providers(vec![ConfigRoot::new(), TempRoot::new()])
    // Elicitation
    .elicitation(OnboardingElicitation::new())
    .elicitations(vec![SurveyElicitation::new(), FeedbackElicitation::new()])
    // Notifications
    .notification_provider(ProgressNotification::new())
    .notification_providers(vec![AlertNotification::new(), StatusNotification::new()])
    // Server configuration
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;
```

### Complete MCP Implementation
**All areas implemented with fine-grained trait architecture:**

- ✅ **Tools** (`ToolDefinition`) - Dynamic tool execution with validation, schema generation, and metadata
- ✅ **Resources** (`ResourceDefinition`) - Static and dynamic content serving with access control
- ✅ **Prompts** (`PromptDefinition`) - AI interaction template generation with parameter validation
- ✅ **Completion** (`CompletionDefinition`) - Context-aware text completion with model preferences
- ✅ **Notifications** (`NotificationDefinition`) - Real-time SSE event broadcasting with filtering
- ✅ **Elicitation** (`ElicitationDefinition`) - Structured user input collection with validation (rides MRTR on 2026)
- ⚠️ **Logging** (`LoggerDefinition`) - Dynamic log level management — *deprecated on 2026 (SEP-2577); per-request `logLevel` opt-in*
- ⚠️ **Roots** (`RootDefinition`) - Secure file system access boundaries — *deprecated on 2026 (SEP-2577); rides MRTR*
- ⚠️ **Sampling** (`SamplingDefinition`) - AI model integration patterns — *deprecated on 2026 (SEP-2577); rides MRTR*
- ✅ **Tasks** — Tasks extension on 2026 (`turul-mcp-ext-tasks`); core capability on the 2025-11-25 opt-in
- ✅ **Session Management** - Stateful operations with UUID v7 — *2025-11-25 opt-in only; the 2026 core is stateless*

### Transport Support
- **Streamable HTTP** - Production transport via `StreamableHttpHandler` (HTTP/1.1 & HTTP/2 with chunked SSE; stateless on the 2026-07-28 default, stateful with GET SSE on the 2025-11-25 opt-in)
- **HTTP+SSE (Legacy)** - Backward-compatible transport via `SessionMcpHandler` (protocol <= 2024-11-05)
- **AWS Lambda** - Serverless deployment with streaming responses
- **Stdio** - Planned for future implementation

> **Note**: The framework auto-selects transport handler based on protocol version negotiation.

## 📚 Examples Overview

### 🏢 Real-World Business Applications
Development servers for actual business problems:

1. **audit-trail-server** → Application Audit & Compliance System (SQLite-backed)
2. **elicitation-server** → Customer Onboarding Platform
3. **notification-server** → Development Team Alert System
4. **completion-server** → IDE Auto-Completion Server
5. **prompts-server** → AI-Assisted Development Prompts
6. **derive-macro-server** → Code Generation & Template Engine
7. **calculator-add-\*-server** → Calculator examples (builder, function, derive, manual patterns)
8. **resources-server** → Development Team Resource Hub

### 🔧 Framework Demonstrations
Educational examples showcasing framework patterns:
- **Basic Patterns**: minimal-server, calculator-add-manual-server, zero-config-getting-started
- **2026 Protocol Features**: ext-tasks-server (Tasks extension), mrtr-elicitation-server (multi-round-trip input), origin-policy-server (DNS-rebinding protection), header-bound-tools-server (SEP-2243 `Mcp-Param`), streamable-http-client (paired 2026 client)
- **Advanced Features**: stateful-server (2025-pinned), pagination-server, tasks-e2e-inmemory-server (2025-pinned)
- **Macro System**: derive-macro-server, function-macro-server, function-resource-server
- **Serverless**: lambda-mcp-server (AWS Lambda with SQS integration)

## ☁️ Serverless Support

### AWS Lambda MCP Server
Full serverless implementation with advanced AWS integration:

```bash
cd examples/lambda-mcp-server

# Local development
cargo lambda watch

# Deploy to AWS
cargo lambda build --release
sam deploy --guided
```

**Features:**
- 🔄 Dual event sources (HTTP + SQS)
- 📡 200MB streaming responses
- 🗄️ DynamoDB session management
- ⚡ Sub-200ms cold starts
- 📊 CloudWatch + X-Ray integration

## 🧪 Testing & Quality

### 🧪 **Comprehensive Test Coverage - Development Quality**

**Framework Excellence**: broad test coverage across all components with complete async SessionContext integration:

- **✅ Core Framework Tests** - Protocol, server, client, derive macros
- **✅ SessionContext Integration** - Full session state management
- **✅ Framework Integration Tests** - Proper API usage patterns
- **✅ MCP Compliance Tests** - Protocol specification validation
- **✅ Builder Pattern Tests** - Runtime tool creation
- **✅ E2E Integration Tests** - Streamable HTTP, SSE, task lifecycle
- **✅ Example Applications** - Real-world scenario validation

```bash
# Run the default-members (2026-07-28) test suite
cargo test --workspace

# SessionContext integration tests
cargo test -p turul-mcp-framework-integration-tests --test session_context_macro_tests

# Framework integration tests (proper patterns)
cargo test -p turul-mcp-framework-integration-tests --test feature_tests

# MCP compliance tests
cargo test -p turul-mcp-framework-integration-tests --test compliance
```

### 🎯 **Framework-Native Testing Patterns**

**The RIGHT way to test MCP applications** - Use framework APIs, not raw JSON:

```rust
// ✅ CORRECT: Framework integration test
use turul_mcp_server::prelude::*;
use turul_mcp_derive::McpTool;

#[derive(McpTool, Default)]
#[tool(name = "calculator", description = "Add numbers")]
struct Calculator {
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.a + self.b)
    }
}

#[tokio::test]
async fn test_calculator_tool() {
    let tool = Calculator { a: 5.0, b: 3.0 };
    
    // Use framework's McpTool trait
    let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await.unwrap();
    
    // Verify using framework result types
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        ToolResult::Text { text, .. } => {
            let parsed: Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["output"], 8.0); // Derive macro uses "output"
        }
        _ => panic!("Expected text result")
    }
}

#[tokio::test] 
async fn test_server_integration() {
    // Use framework builders
    let server = McpServer::builder()
        .name("test-server")
        .tool(Calculator::default())
        .build()
        .unwrap();
    
    // Server builds successfully with proper type checking
    assert!(true);
}
```

**❌ WRONG: Raw JSON manipulation** (old problematic pattern):
```rust
// DON'T DO THIS - mixing incompatible JSON-RPC types
let request = json!({
    "method": "tools/call",
    "params": { "name": "calc" }
});
```

### 🔄 **SessionContext Integration - Fully Working**

**Complete session state management** with proper test infrastructure:

```rust
// SessionContext integration test
use crate::test_helpers::create_test_session;

#[tokio::test]
async fn test_session_state_management() {
    let session = create_test_session().await;
    
    // Session state works perfectly
session.set_typed_state("counter", &42i32).await.unwrap();
    let value: i32 = session.get_typed_state("counter").await.unwrap();
    assert_eq!(value, 42);
    
    // Progress notifications work
    session.notify_progress("processing", 50).await;
    
    // Tool execution with session context
    let tool = Calculator { a: 1.0, b: 2.0 };
    let result = tool.call(json!({"a": 1.0, "b": 2.0}), Some(session)).await.unwrap();
    assert_eq!(result.content.len(), 1);
}
```

**Test Infrastructure Available**:
- `TestSessionBuilder` - Create real SessionContext instances
- `TestNotificationBroadcaster` - Verify notifications  
- `create_test_session()` - Helper for simple cases
- Full storage backend integration

## 🎯 Development Patterns

### 1. Function Macros (Recommended for Simplicity)
**Best for:** Quick development, natural syntax, minimal boilerplate

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "weather", description = "Get weather information")]
async fn get_weather(
    #[param(description = "City name")] city: String,
    #[param(description = "Temperature unit")] unit: Option<String>,
) -> McpResult<String> {
    let unit = unit.unwrap_or_else(|| "celsius".to_string());
    Ok(format!("Weather in {}: 22°{}", city, if unit == "fahrenheit" { "F" } else { "C" }))
}

// Usage in server
let server = McpServer::builder()
    .name("weather-server")
    .version("1.0.0")
    .tool_fn(get_weather)
    .build()?;
```

### 2. Derive Macros (Struct-Based)
**Best for:** Complex tools, organized codebases, multiple related functions

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone)]
#[tool(name = "file_manager", description = "File management operations")]
struct FileManager {
    #[param(description = "Operation (create, read, delete)")]
    operation: String,
    #[param(description = "File path")]
    path: String,
    #[param(description = "File content (for create operation)")]
    content: Option<String>,
}

impl FileManager {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        match self.operation.as_str() {
            "create" => {
                let content = self.content.as_ref().unwrap_or(&"Empty file".to_string());
                Ok(format!("Created file '{}' with content: {}", self.path, content))
            },
            "read" => Ok(format!("Reading file: {}", self.path)),
            "delete" => {
                if let Some(session) = session {
                    session.notify_progress(&format!("Deleting {}", self.path), 100).await;
                }
                Ok(format!("Deleted file: {}", self.path))
            },
            _ => Err("Invalid operation".into()),
        }
    }
}

// Usage in server
let server = McpServer::builder()
    .name("file-server")
    .version("1.0.0")
    .tool(FileManager {
        operation: "create".to_string(),
        path: "/tmp/example".to_string(),
        content: None,
    })
    .build()?;
```

### 3. Builder Pattern (Runtime Flexibility)
**Best for:** Dynamic tools, configuration-driven systems

```rust
use turul_mcp_server::prelude::*;
use serde_json::json;

let multiply_tool = ToolBuilder::new("multiply")
    .description("Multiply two numbers")
    .number_param("a", "First number")
    .number_param("b", "Second number")
    .number_output() // Generates {"result": number} schema
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'b'")?;
        
        Ok(json!({"result": a * b}))
    })
    .build()
    .map_err(|e| format!("Failed to build tool: {}", e))?;

// Usage in server
let server = McpServer::builder()
    .name("calculator-server")
    .version("1.0.0")
    .tool(multiply_tool)
    .build()?;
```

### 4. Manual Implementation (Maximum Control)
**Best for:** Performance optimization, custom behavior

```rust
use turul_mcp_server::prelude::*;  // Re-exports builders prelude + framework traits
use turul_mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

struct ManualTool;

impl HasBaseMetadata for ManualTool {
    fn name(&self) -> &str { "manual_tool" }
}

impl HasDescription for ManualTool {
    fn description(&self) -> Option<&str> { Some("Manual implementation with full control") }
}

impl HasInputSchema for ManualTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            ToolSchema::object()
                .with_properties(HashMap::from([
                    ("input".to_string(), JsonSchema::string())
                ]))
        })
    }
}

impl HasOutputSchema for ManualTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for ManualTool {
    fn annotations(&self) -> Option<&ToolAnnotations> { None }
}

impl HasToolMeta for ManualTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

impl HasIcons for ManualTool {}     // No icons
impl HasExecution for ManualTool {} // No task support

#[async_trait]
impl McpTool for ManualTool {
    async fn call(&self, _args: Value, _session: Option<SessionContext>) 
        -> McpResult<CallToolResult> {
        // Full control over implementation
        Ok(CallToolResult::success(vec![
            ToolResult::text("Manual tool with complete control")
        ]))
    }
}

// Usage in server
let server = McpServer::builder()
    .name("manual-server")
    .version("1.0.0")
    .tool(ManualTool)
    .build()?;
```

## 🔧 Client Library

Comprehensive MCP client for HTTP transport:

```rust
use turul_mcp_client::{McpClient, McpClientBuilder, transport::HttpTransport};
use std::time::Duration;

// Create HTTP transport
let transport = HttpTransport::new("http://localhost:8080/mcp")?;

// Create client using builder pattern
let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .build();

// Connect — negotiates the wire spec per connection
// (2026-07-28 `server/discover` first, falls back to 2025-11-25 `initialize`)
client.connect().await?;

// List available tools
let tools = client.list_tools().await?;

// Call a tool
let result = client.call_tool("add", json!({
    "a": 10.0,
    "b": 20.0
})).await?;

// List and read resources
let resources = client.list_resources().await?;
let content = client.read_resource("config://app.json").await?;
```

## 🚀 Performance Features

### Modern Architecture
- **UUID v7** - Time-ordered IDs for better database performance and observability
- **Workspace Dependencies** - Consistent dependency management across the framework crates and examples
- **Rust 2024 Edition** - Latest language features and performance improvements
- **Tokio/Hyper** - High-performance async runtime with HTTP/2 support

### Development Quality
- **Session Management** - Automatic cleanup and state persistence
- **Real-time Notifications** - SSE-based event streaming
- **CORS Support** - Browser client compatibility
- **Comprehensive Logging** - Structured logging with correlation IDs
- **Error Handling** - Detailed error types with recovery strategies

## 🔍 MCP Protocol Compliance

**Default build targets MCP 2026-07-28 (stateless); 2025-11-25 is opt-in:**

- ✅ **JSON-RPC 2.0** - Complete request/response with `_meta` fields
- ✅ **Stateless core (2026-07-28)** - `server/discover`, per-request `_meta` capability negotiation, no `Mcp-Session-Id`
- ✅ **Progress Tracking** - Long-running operation support
- ✅ **Cursor Pagination** - Efficient large dataset navigation
- ✅ **Caching headers (2026-07-28)** - `ttlMs` / `cacheScope` on list/read results
- ✅ **Transport Agnostic** - Multiple transport implementations

### Testing Your Server (2026-07-28 default)
```bash
# Test tool execution — capabilities ride in per-request `_meta`, no session header
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/call" \
  -H "Mcp-Name: add" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {"a": 10, "b": 20},
      "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
      }
    }
  }'
```

> The 2026 core is stateless. Sessions, the `Mcp-Session-Id` header, and the session
> lifecycle (DELETE termination, TTL/expiry) are part of the **2025-11-25 opt-in** lane;
> the `client-initialise-server` + `session-management-compliance-test` packages exercise
> them under that pin.

## 🛠️ Development & Testing

### Building the Framework

```bash
# Build all workspace crates
cargo build

# Build with release optimizations
cargo build --release
```

### Running Tests

The framework includes a **comprehensive test suite** covering all functionality. Test server binaries are **automatically built** when needed - no manual setup required.

```bash
# Run all tests (recommended - includes E2E integration tests)
cargo test --workspace

# Run the full lane-by-lane gate suite (2026 default + 2025 opt-in matrix)
./scripts/ci-gates.sh all

# Run a 2026 wire-acceptance suite directly
cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test discover_stateless_2026

# Run with logging output
RUST_LOG=info cargo test --workspace
```

**Key Features:**
- ✅ **Auto-build test servers** - Missing test binaries are built automatically on first test run
- ✅ **Zero configuration** - Just run `cargo test` and everything works
- ✅ **Clean workspace support** - `cargo clean && cargo test` works without manual steps

The test infrastructure automatically builds required test server binaries (`resource-test-server`, `prompts-test-server`, `tools-test-server`, etc.) when running integration tests. This ensures a seamless developer experience.

### Notifications & Streaming

On the 2026 default, notifications ride **POST** SSE — request-scoped notifications
(`notifications/progress`, `notifications/message`) flow on the originating request's
own response stream, and server-push subscriptions use the long-lived
`subscriptions/listen` POST stream. The 2025-era GET-SSE endpoint and
`Mcp-Session-Id`-keyed streams are part of the 2025-11-25 opt-in lane (the HTTP+SSE
transport is deprecated upstream, SEP-2596).

- **Live demo**: `cargo run -p notification-server`, then drive it with
  `cargo run -p streamable-http-client http://127.0.0.1:8005/mcp` (opens a
  `subscriptions/listen` stream and triggers list-changed + resource-update deliveries).
- **Wire tests**: `cargo test -p turul-mcp-server --no-default-features --features http,sse,protocol-2026-07-28 --test subscriptions_listen_2026`

## 📊 Business Value Examples

### Enterprise Integration
- **audit-trail-server**: SOX, PCI DSS, GDPR, and HIPAA compliance reporting

### Developer Productivity
- **completion-server**: Context-aware IDE completions for multiple languages and frameworks
- **prompts-server**: AI-powered code review and architecture guidance
- **derive-macro-server**: Template-based code generation with validation

### Customer Experience
- **elicitation-server**: GDPR-compliant customer onboarding with regulatory forms
- **notification-server**: Real-time incident management with escalation workflows

## 🛡️ Security & Reliability

- **Memory Safety** - Rust's ownership system prevents common vulnerabilities
- **Type Safety** - Compile-time validation with automatic schema generation
- **Input Validation** - Parameter constraints and sanitization
- **Session Isolation** - Secure multi-tenant operation
- **Audit Logging** - Comprehensive activity tracking with UUID v7 correlation
- **Resource Limits** - Configurable timeouts and memory constraints

## 🤝 Contributing

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Add tests** for your changes
4. **Run** the full test suite (`cargo test --workspace`)
5. **Benchmark** performance impact if applicable
6. **Commit** changes (`git commit -m 'Add amazing feature'`)
7. **Push** to branch (`git push origin feature/amazing-feature`)
8. **Open** a Pull Request

## 📝 License

This project is licensed under the MIT OR Apache-2.0 License - see the LICENSE files for details.

## 🙏 Acknowledgments

- **[Model Context Protocol](https://modelcontextprotocol.io)** - The foundational specification
- **[Tokio](https://tokio.rs)** - Async runtime powering the framework
- **[Hyper](https://hyper.rs)** - HTTP foundation with HTTP/2 support
- **[Serde](https://serde.rs)** - Serialization framework
- **Rust Community** - For exceptional tooling and ecosystem

## 📋 Development Status & Current Limitations

### 🎯 Current Framework State
- **MCP 2026-07-28**: default build targets the current stateless spec; 2025-11-25 remains fully supported as the opt-in stateful line
- **Examples Validated**: 54 active examples compile under their lane's CI gates (2026-07-28 default lane plus the pinned 2025-11-25 regression set — see EXAMPLES.md)
- **SSE Streaming Verified**: Real-time notifications and session-aware logging working correctly
- **Pre-1.0 (0.4.x)**: production-shaped with comprehensive test coverage; public APIs may still change before 1.0.0

### 🚧 Current Limitations

**Transport & Streaming:**
- **Lambda SSE**: Snapshot-based responses via `handle()`, real-time streaming via `run_streaming()` / `run_streaming_with()` with graceful completion-invocation handling
- **Additional transport variants**: Streamable HTTP and legacy HTTP+SSE are supported; stdio remains planned
- **CI Environment Testing**: SSE tests require port binding capabilities (graceful fallbacks implemented)

**Features & Integration:**
- **Resource Subscriptions**: on the 2026-07-28 default, resource-update notifications ride the `subscriptions/listen` stream (`resources.subscribe` is advertised); the 2025-11-25 opt-in path does not implement the legacy `resources/subscribe` RPC
- **Authentication Middleware**: OAuth 2.1 Resource Server support via `turul-mcp-oauth` (JWT validation, Bearer token middleware, `.well-known` metadata)
- **Cross-platform Compatibility**: Primarily tested on Linux development environments

### 📊 Areas for Enhancement
- **Performance Monitoring**: Basic benchmarks available, comprehensive monitoring planned
- **Concurrency Stress Testing**: Some resource tests show occasional failures under extreme load
- **Browser Compatibility**: CORS support available but may need tuning for specific client requirements

**Framework Philosophy**: We prioritize honest documentation over inflated claims. The limitations above reflect our commitment to transparency about the current development state.

---

**🚀 Ready to build MCP servers?** Start with our [comprehensive examples](examples/) or check the [getting started guide](EXAMPLES.md).

**💡 Need help?** Open an issue or check our [examples](examples/) covering everything from simple calculators to enterprise systems.
