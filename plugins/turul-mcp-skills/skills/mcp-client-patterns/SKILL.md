---
name: mcp-client-patterns
description: >
  This skill should be used when the user asks about "MCP client",
  "McpClient", "McpClientBuilder", "connect to MCP server",
  "HttpTransport", "SseTransport", "tool call from client",
  "client session", "client task workflow", "ToolCallResponse",
  "client error handling", "disconnect", "client configuration",
  "refresh_tools", "tool change notification", or "list_changed client".
  Covers transport selection, connection lifecycle, tool/resource/prompt
  invocation, task workflows, tool change notifications, and error handling
  for the Turul MCP Client (turul-mcp-client crate, Rust). Do NOT use for
  server-side work (tools, resources, prompts) — see tool-creation-patterns
  and resource-prompt-patterns.
---

# Turul MCP Client Patterns

**Spec lane: MCP 2026-07-28 (current default, `client-bilingual` feature).** `turul-mcp-client` links both versioned protocol crates and negotiates the wire spec **per connection**, at `connect()` time — it is not a build-time choice between two clients. `--no-default-features --features client-2025-11-25-only` or `client-2026-07-28-only` narrow a build to exactly one wire spec if you need that. Everything below describes bilingual (default) behavior; where the two lanes diverge, both are called out.

## Transport Selection

Transport is auto-detected from the URL. Transport and wire-spec negotiation are independent: `HttpTransport` carries either 2025-11-25 (Streamable HTTP, stateful) or 2026-07-28 (stateless core) depending on what `connect()` negotiates.

```
McpClientBuilder::new().with_url(url)?
├─ URL contains /sse or ?transport=sse ──→ SseTransport (Legacy HTTP+SSE, MCP 2024-11-05 — no 2026-07-28 support)
└─ Otherwise (default) ─────────────────→ HttpTransport (negotiates 2026-07-28 or 2025-11-25 at connect() time)
```

Or build explicitly:

```rust
// Auto-detect (recommended)
McpClientBuilder::new().with_url("http://host/mcp")?

// Explicit HTTP (negotiates 2026-07-28 stateless core, or falls back to 2025-11-25 Streamable HTTP)
McpClientBuilder::new().with_transport(Box::new(HttpTransport::new("http://host/mcp")?))

// Explicit SSE (Legacy HTTP+SSE, MCP 2024-11-05 only — never negotiates 2026-07-28)
McpClientBuilder::new().with_transport(Box::new(SseTransport::new("http://host/sse")?))
```

| Feature | `HttpTransport` | `SseTransport` |
|---|---|---|
| Protocol | 2026-07-28 (stateless core) or 2025-11-25 (Streamable HTTP), negotiated per connection | MCP 2024-11-05 (Legacy HTTP+SSE) only |
| Server events | SSE streaming on response | Separate SSE endpoint |
| Session management | `Mcp-Session-Id` header on 2025-11-25 connections only; absent entirely on 2026-07-28 | `Mcp-Session-Id` header |
| Recommended for | New servers | Legacy 2024-11-05 servers only |

See [references/transport-guide.md](references/transport-guide.md) for full details.

## Quick Start

```rust
// turul-mcp-client v0.4
use turul_mcp_client::{McpClientBuilder, McpClientResult};
use serde_json::json;

#[tokio::main]
async fn main() -> McpClientResult<()> {
    // Build client — transport auto-detected from URL
    let client = McpClientBuilder::new()
        .with_url("http://localhost:8080/mcp")?
        .build();

    // Connect — negotiates the wire spec: probes `server/discover` first;
    // a 2026-07-28 server answers and the connection stays stateless (no
    // initialize, no Mcp-Session-Id). If the probe indicates a legacy
    // server, the client falls back to the 2025-11-25 initialize handshake.
    client.connect().await?;

    // Use the client — identical call surface regardless of which spec was negotiated
    let tools = client.list_tools().await?;
    let result = client.call_tool("add", json!({"a": 1, "b": 2})).await?;
    println!("{result:?}");

    // Clean up. Sends DELETE only if a session was established (2025-11-25
    // fallback); a no-op on the wire for a 2026-07-28 stateless connection.
    client.disconnect().await?;
    Ok(())
}
```

For custom client identity, build a `ClientConfig`:

```rust
// turul-mcp-client v0.4
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
| `call_tool(name, args)` | `&str, Value` | `McpClientResult<CallToolResult>` |
| `call_tool_with_task(name, args, ttl)` | `&str, Value, Option<i64>` | `McpClientResult<ToolCallResponse>` |
| `list_resources()` | `&self` | `McpClientResult<Vec<Resource>>` |
| `list_resource_templates()` | `&self` | `McpClientResult<Vec<ResourceTemplate>>` |
| `read_resource(uri)` | `&str` | `McpClientResult<Vec<ResourceContent>>` |
| `list_prompts()` | `&self` | `McpClientResult<Vec<Prompt>>` |
| `get_prompt(name, args)` | `&str, Option<Value>` | `McpClientResult<GetPromptResult>` |
| `get_task(id)` | `&str` | `McpClientResult<Task>` |
| `get_task_result(id)` | `&str` | `McpClientResult<Value>` (blocks until terminal) |
| `cancel_task(id)` | `&str` | `McpClientResult<Task>` |
| `ping()` | `&self` | `McpClientResult<()>` |

Additional methods: `list_resources_paginated()`, `list_resource_templates_paginated()`, `list_prompts_paginated()`, `list_tasks()`, `list_tasks_paginated()`, `connection_status()`, `session_info()`, `transport_stats()`.

## Task Workflow

`call_tool_with_task()` returns a `ToolCallResponse` enum — either the result immediately or a task handle for long-running operations.

```rust
// turul-mcp-client v0.4
use turul_mcp_client::ToolCallResponse;
use turul_mcp_protocol::TaskStatus;

let response = client.call_tool_with_task("slow_add", json!({"a": 1, "b": 2}), None).await?;

match response {
    ToolCallResponse::Immediate(result) => {
        println!("Immediate result: {result:?}");
    }
    ToolCallResponse::TaskCreated(task) => {
        // Option A: Block until terminal (per MCP spec)
        let value = client.get_task_result(&task.task_id).await?;

        // Option B: Poll for status
        loop {
            let task = client.get_task(&task.task_id).await?;
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
// turul-mcp-client v0.4
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

**Session status codes — 2025-11-25 fallback connections only.** These only apply once a connection has negotiated down to 2025-11-25 Streamable HTTP; a 2026-07-28 stateless connection has no `Mcp-Session-Id` and no session-expiry 404 to recover from.

| HTTP Status | Meaning | Client Action |
|---|---|---|
| **401** | Missing or invalid auth token, OR missing `Mcp-Session-Id` header | Re-authenticate or add session header |
| **404** | Session ID not found or terminated | Start fresh `initialize` handshake (do NOT re-authenticate) |
| **500** | Server internal error | Retry with backoff |

The 401 vs 404 distinction matters: 404 means "your session is gone, create a new one" — not an auth problem. `McpClient` handles this automatically on 2025-11-25 connections: `is_session_expired()` resets local session state, clears the stale `Mcp-Session-Id`, re-runs `initialize`, and retries the original request.

**Missing-resource error code.** 2026-07-28 reuses the JSON-RPC standard `-32602` for "not found" where 2025-11-25 used `-32002`, and forbids a 2026 implementation from emitting `-32002` at all. `McpClientError::is_resource_not_found(version)` therefore takes the connection's negotiated version: on 2025-11-25 it accepts both codes, on 2026-07-28 only `-32602`. Accepting both unconditionally would read a 2026 permission denial as a missing resource.

See [references/error-handling-guide.md](references/error-handling-guide.md) for full variant catalog and retry patterns.

## Tool Change Notifications

When a server uses `ToolChangeMode::Dynamic`, clients receive `notifications/tools/list_changed`. The client caches tools and auto-invalidates on notification:

```rust
// turul-mcp-client v0.4
// list_tools() returns cached tools; refresh_tools() forces a fresh tools/list call
let tools = client.list_tools().await?;         // Cached (fast)
let tools = client.refresh_tools().await?;      // Forces fresh fetch

// After server emits notifications/tools/list_changed,
// the next list_tools() call automatically fetches fresh data.
```

For a complete dynamic tools E2E example, see `examples/dynamic-tools-test-client`.

## Common Mistakes

1. **Forgetting `client.connect().await?` before operations** — negotiation never ran, all calls fail with `SessionError::NotInitialized`
2. **Not calling `client.disconnect().await?`** — on a 2025-11-25 fallback connection the server session leaks (Drop spawns cleanup but is best-effort); on 2026-07-28 there's no server-side session to leak, but local resources (event listener task) still need tearing down
3. **Using `SseTransport` against a 2026-07-28 server** — `SseTransport` only speaks legacy 2024-11-05 and never negotiates 2026-07-28 or 2025-11-25; use `HttpTransport` or let `with_url()` auto-detect
4. **Not handling `ToolCallResponse::TaskCreated`** — `call_tool_with_task()` can return a task handle instead of immediate result
5. **Not checking `error.is_retryable()`** — retrying non-retryable errors wastes time and hides bugs
6. **Hardcoding URLs** — make server endpoint configurable via environment or config
7. **Using default `ClientInfo`** — server logs show generic "mcp-client"; set `name` and `version` for debuggability
8. **Blocking inside async client operations** — use `tokio::time::sleep`, not `std::thread::sleep`

## Authentication & Discovery

This skill covers transport and lifecycle, not authentication. MCP clients that connect to OAuth-protected resource servers need to handle the authorization flow:

1. **401 + WWW-Authenticate** — RS rejects unauthenticated requests with a `WWW-Authenticate: Bearer` header containing a `resource_metadata` URL
2. **Protected Resource Metadata** — client fetches `/.well-known/oauth-protected-resource` from the RS to discover `authorization_servers`
3. **AS Metadata** — client fetches `/.well-known/oauth-authorization-server` from the AS to discover endpoints
4. **Authorization code + PKCE** — client runs the PKCE flow, including `resource` parameter at both `/authorize` and `/token`
5. **Bearer token** — client includes the access token in subsequent RS requests

The `turul-mcp-client` crate handles transport and session lifecycle. Token acquisition and header injection are the client application's responsibility.

**See:** the `auth-patterns` skill for RS-side validation and the `authorization-server-patterns` skill for building a demo AS to test against.

## Bearer Lifecycle & Rotation

Per MCP authorization, the bearer token MUST be present on **every** HTTP request a client makes — that includes the POST request stream, the GET SSE listener, and the DELETE cleanup. A long-lived client that holds a token across rotations needs to swap the token in place rather than rebuild the whole client (which would drop the connection pool).

```rust
// turul-mcp-client v0.4
// Rotate the bearer on a long-lived client without losing the connection pool.
// All five outbound surfaces (POST, GET SSE, DELETE, send_request_with_headers,
// send_notification) pick up the new token immediately.
client.set_bearer(Some(&fresh_token)).await;
client.disconnect().await?;  // DELETE goes out with the fresh bearer
```

**Lifecycle invariants worth knowing** (v0.3.33–v0.3.46 hardening):

| Invariant | Notes |
|---|---|
| `disconnect()` is idempotent | Safe to call multiple times; also fires implicitly from `Drop`. After explicit `disconnect()`, the implicit `Drop` is a no-op (no duplicate DELETE). |
| Concurrent `call_tool` on `Arc<McpClient>` runs in parallel | `Transport` trait takes `&self` on hot paths; reqwest's connection pool serves concurrent requests. No outer `Mutex`. |
| SSE GET 4xx is terminal (2025-11-25 fallback only) | The background SSE listener treats any 4xx on the GET stream as a permanent failure, clears the cached session ID, and exits. The next `connect()` re-establishes from a fresh `initialize`. The standalone GET SSE endpoint this depends on is removed entirely in 2026-07-28 — on a negotiated 2026-07-28 connection, server-initiated messages ride the response stream of the originating request instead. |
| `set_bearer(None)` falls back to default headers | Useful for clearing an override and reverting to whatever was baked into `ClientBuilder::default_headers()`. |

Common rotation pattern for OAuth `client_credentials` deployments:

```rust
loop {
    let token = oauth.refresh().await?;     // get fresh bearer
    client.set_bearer(Some(&token)).await;  // swap in place
    tokio::time::sleep(token.ttl - skew).await;
}
```

## Beyond This Skill

- **Server-side tool creation** — use the `tool-creation-patterns` skill
- **Output schemas and structuredContent** — use the `output-schemas` skill
- **Server configuration and builder** — see [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)
- **MCP protocol compliance** — see `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md`
- **OAuth RS validation** — see the `auth-patterns` skill for `JwtValidator`, audience validation, and RFC 9728 metadata
- **Demo Authorization Server** — see the `authorization-server-patterns` skill for building a test AS
