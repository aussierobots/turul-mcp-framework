---
name: mcp-client-patterns
description: >
  This skill should be used when the user asks about "MCP client",
  "McpClient", "McpClientBuilder", "connect to MCP server",
  "HttpTransport", "SseTransport", "tool call from client",
  "client session", "client task workflow", "ToolCallResponse",
  "client error handling", "disconnect", or "client configuration".
  Covers transport selection, connection lifecycle, tool/resource/prompt
  invocation, task workflows, and error handling for the Turul MCP Client
  (turul-mcp-client crate, Rust).
---

# Turul MCP Client Patterns

## Transport Selection

Transport is auto-detected from the URL:

```
McpClientBuilder::new().with_url(url)?
├─ URL contains /sse or ?transport=sse ──→ SseTransport (Legacy HTTP+SSE, MCP 2024-11-05)
└─ Otherwise (default) ─────────────────→ HttpTransport (Streamable HTTP, MCP 2025-11-25)
```

Or build explicitly:

```rust
// Auto-detect (recommended)
McpClientBuilder::new().with_url("http://host/mcp")?

// Explicit HTTP (Streamable HTTP, MCP 2025-11-25)
McpClientBuilder::new().with_transport(Box::new(HttpTransport::new("http://host/mcp")?))

// Explicit SSE (Legacy HTTP+SSE, MCP 2024-11-05)
McpClientBuilder::new().with_transport(Box::new(SseTransport::new("http://host/sse")?))
```

| Feature | `HttpTransport` | `SseTransport` |
|---|---|---|
| Protocol | MCP 2025-11-25 (Streamable HTTP) | MCP 2024-11-05 (Legacy HTTP+SSE) |
| Server events | SSE streaming on response | Separate SSE endpoint |
| Session management | `Mcp-Session-Id` header | Separate SSE connection |
| Recommended for | New servers | Legacy servers only |

See [references/transport-guide.md](references/transport-guide.md) for full details.

## Quick Start

```rust
// turul-mcp-client v0.3
use turul_mcp_client::{McpClientBuilder, McpClientResult};
use serde_json::json;

#[tokio::main]
async fn main() -> McpClientResult<()> {
    // Build client — transport auto-detected from URL
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .build();

    // Connect (performs initialize handshake)
    client.connect().await?;

    // Use the client
    let tools = client.list_tools().await?;
    let result = client.call_tool("add", json!({"a": 1, "b": 2})).await?;
    println!("{result:?}");

    // Clean up (sends DELETE to server)
    client.disconnect().await?;
    Ok(())
}
```

For custom client identity, build a `ClientConfig`:

```rust
// turul-mcp-client v0.3
use turul_mcp_client::config::{ClientConfig, ClientInfo};

let config = ClientConfig {
    client_info: ClientInfo {
        name: "my-app".into(),
        version: "1.0.0".into(),
        ..Default::default()
    },
    ..Default::default()
};

let client = McpClientBuilder::new()
    .with_url("http://localhost:8080/mcp")?
    .with_config(config)
    .build();
```

## Core/Common Operations

These are the most frequently used methods. For the full API surface, see the `McpClient` source in `crates/turul-mcp-client/src/client.rs`.

| Method | Parameters | Returns |
|---|---|---|
| `connect()` | `&self` | `McpClientResult<()>` |
| `disconnect()` | `&self` | `McpClientResult<()>` |
| `is_ready()` | `&self` | `bool` |
| `list_tools()` | `&self` | `McpClientResult<Vec<Tool>>` |
| `list_tools_paginated(cursor)` | `Option<Cursor>` | `McpClientResult<ListToolsResult>` |
| `call_tool(name, args)` | `&str, Value` | `McpClientResult<Vec<ToolResult>>` |
| `call_tool_with_task(name, args, ttl)` | `&str, Value, Option<i64>` | `McpClientResult<ToolCallResponse>` |
| `list_resources()` | `&self` | `McpClientResult<Vec<Resource>>` |
| `read_resource(uri)` | `&str` | `McpClientResult<Vec<ResourceContent>>` |
| `list_prompts()` | `&self` | `McpClientResult<Vec<Prompt>>` |
| `get_prompt(name, args)` | `&str, Option<Value>` | `McpClientResult<Vec<PromptMessage>>` |
| `get_task(id)` | `&str` | `McpClientResult<Task>` |
| `get_task_result(id)` | `&str` | `McpClientResult<Value>` (blocks until terminal) |
| `cancel_task(id)` | `&str` | `McpClientResult<Task>` |
| `ping()` | `&self` | `McpClientResult<()>` |

Additional methods: `list_resources_paginated()`, `list_prompts_paginated()`, `list_tasks()`, `list_tasks_paginated()`, `connection_status()`, `session_info()`, `transport_stats()`.

## Task Workflow

`call_tool_with_task()` returns a `ToolCallResponse` enum — either the result immediately or a task handle for long-running operations.

```rust
// turul-mcp-client v0.3
use turul_mcp_client::ToolCallResponse;
use turul_mcp_protocol::TaskStatus;

let response = client.call_tool_with_task("slow_add", json!({"a": 1, "b": 2}), None).await?;

match response {
    ToolCallResponse::Immediate(result) => {
        println!("Immediate result: {result:?}");
    }
    ToolCallResponse::TaskCreated(task) => {
        let task_id = &task.id;
        // Option A: Block until terminal (per MCP spec)
        let value = client.get_task_result(task_id).await?;

        // Option B: Poll for status
        loop {
            let task = client.get_task(task_id).await?;
            match task.status {
                TaskStatus::Working => { /* continue polling */ }
                TaskStatus::Completed => { break; }
                TaskStatus::Failed => { eprintln!("Task failed"); break; }
                TaskStatus::InputRequired => {
                    // Server needs client input (elicitation).
                    // McpClient does not yet expose an elicitation response API.
                    // Handle at application level or via raw JSON-RPC.
                    eprintln!("Task requires input — not yet supported by McpClient");
                    break;
                }
                TaskStatus::Cancelled => { break; }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
```

**Helper methods on `ToolCallResponse`:**
- `response.is_task()` — returns `true` if `TaskCreated`
- `response.task()` — returns `Option<&Task>`
- `response.immediate_result()` — returns `Option<&CallToolResult>`

**Known gap:** `McpClient` does not expose an elicitation response method. `InputRequired` status must be handled at the application level or via raw JSON-RPC.

See [examples/task-workflow.rs](examples/task-workflow.rs) for a complete example.

## Configuration

`ClientConfig` is a nested struct — all fields have sensible defaults.

```rust
// turul-mcp-client v0.3
use turul_mcp_client::config::*;
use std::time::Duration;

let config = ClientConfig {
    client_info: ClientInfo {
        name: "my-app".into(),
        version: "2.0.0".into(),
        description: Some("My MCP client app".into()),
        vendor: Some("My Company".into()),
        metadata: None,
    },
    timeouts: TimeoutConfig {
        connect: Duration::from_secs(10),       // default: 10s
        request: Duration::from_secs(30),       // default: 30s
        long_operation: Duration::from_secs(300), // default: 300s
        initialization: Duration::from_secs(15), // default: 15s
        heartbeat: Duration::from_secs(30),     // default: 30s
    },
    retry: RetryConfig {
        max_attempts: 3,                        // default: 3
        initial_delay: Duration::from_millis(100), // default: 100ms
        max_delay: Duration::from_secs(10),     // default: 10s
        backoff_multiplier: 2.0,                // default: 2.0
        jitter: 0.1,                            // default: 0.1
        exponential_backoff: true,              // default: true
    },
    connection: ConnectionConfig::default(),
    logging: LoggingConfig::default(),
};
```

Pass via `.with_config(config)` on `McpClientBuilder`.

## Error Handling

`McpClientError` is a nested enum with sub-error types.

| Variant | Sub-errors | Retryable? |
|---|---|---|
| `Transport(TransportError)` | `Http`, `Sse`, `Stdio`, `Unsupported`, `ConnectionFailed`, `Closed` | `ConnectionFailed` + `Closed` yes |
| `Protocol(ProtocolError)` | `InvalidRequest`, `InvalidResponse`, `UnsupportedVersion`, `MethodNotFound`, `InvalidParams`, `NegotiationFailed`, `CapabilityMismatch` | No |
| `Session(SessionError)` | `NotInitialized`, `AlreadyInitialized`, `Expired`, `Terminated`, `InvalidState`, `RecoveryFailed` | No |
| `Connection(reqwest::Error)` | Network errors | Yes |
| `Timeout` | Operation timed out | Yes |
| `ServerError { code, message, data }` | JSON-RPC server error | Codes -32099..-32000 yes |
| `Auth(String)` | Authentication failures | No |
| `Json(serde_json::Error)` | Parse errors | No |
| `Config(String)` | Configuration errors | No |
| `Generic { message }` | Catch-all | No |

**Built-in helpers:**
- `error.is_retryable()` — `true` for transport failures, connection errors, timeouts, retryable server codes
- `error.is_protocol_error()` — `true` for protocol violations
- `error.is_session_error()` — `true` for session lifecycle issues
- `error.error_code()` — extracts JSON-RPC error code if available

Use `RetryConfig::delay_for_attempt(n)` to calculate backoff delay with jitter.

See [references/error-handling-guide.md](references/error-handling-guide.md) for full variant catalog and retry patterns.

## Common Mistakes

1. **Forgetting `client.connect().await?` before operations** — session not initialized, all calls fail with `SessionError::NotInitialized`
2. **Not calling `client.disconnect().await?`** — server session leaks (Drop spawns cleanup but is best-effort)
3. **Using `SseTransport` for MCP 2025-11-25 servers** — wrong protocol; use `HttpTransport` or let `with_url()` auto-detect
4. **Not handling `ToolCallResponse::TaskCreated`** — `call_tool_with_task()` can return a task handle instead of immediate result
5. **Not checking `error.is_retryable()`** — retrying non-retryable errors wastes time and hides bugs
6. **Hardcoding URLs** — make server endpoint configurable via environment or config
7. **Using default `ClientInfo`** — server logs show generic "mcp-client"; set `name` and `version` for debuggability
8. **Blocking inside async client operations** — use `tokio::time::sleep`, not `std::thread::sleep`

## Beyond This Skill

- **Server-side tool creation** — use the `tool-creation-patterns` skill
- **Output schemas and structuredContent** — use the `output-schemas` skill
- **Server configuration and builder** — see [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)
- **MCP protocol compliance** — see [CLAUDE.md — MCP 2025-11-25 Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#mcp-2025-11-25-compliance)
