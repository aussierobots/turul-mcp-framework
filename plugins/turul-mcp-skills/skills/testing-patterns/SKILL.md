---
name: testing-patterns
description: >
  This skill should be used when the user asks about "testing",
  "test patterns", "write tests", "unit test", "e2e test",
  "integration test", "McpTestClient", "TestServerManager",
  "compliance test", "test server", "test fixture", "doctest",
  "cargo test", "test organization", "SSE testing",
  or "test consolidation". Covers unit testing, E2E testing,
  compliance testing, SSE testing, and test organization
  in the Turul MCP Framework (Rust).
---

# Testing Patterns — Turul MCP Framework

The framework uses three testing layers: **unit tests** for individual components, **E2E tests** for full HTTP round-trips via `McpTestClient` + `TestServerManager`, and **compliance tests** for MCP specification conformance. All share common utilities from the `mcp-e2e-shared` crate.

## When to Write Tests

```
What are you testing?
├─ Single tool/resource/prompt logic ────────→ Unit test (#[tokio::test])
├─ Full request→response over HTTP ─────────→ E2E test (TestServerManager + McpTestClient)
├─ MCP specification conformance ───────────→ Compliance test (cargo test --test compliance)
└─ API surface / doc examples ──────────────→ Doctest (```rust block in doc comments)
```

**Default to unit tests.** Only use E2E when you need to verify HTTP transport, session management, or middleware behavior.

## Unit Testing Patterns

Unit tests exercise tools, resources, and prompts directly — no HTTP server needed.

```rust
// turul-mcp-server v0.3
use serde_json::json;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Default)]
#[tool(name = "double", description = "Double a number", output = f64)]
struct DoubleTool {
    #[param(description = "Number to double")]
    n: f64,
}

impl DoubleTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
        Ok(self.n * 2.0)
    }
}

#[tokio::test]
async fn test_double_tool() {
    let tool = DoubleTool { n: 21.0 };
    // Call directly — framework handles JSON conversion in production
    let result = tool.call(json!({"n": 21.0}), None).await.unwrap();
    let value: f64 = serde_json::from_value(result).unwrap();
    assert_eq!(value, 42.0);
}
```

**Key points:**
- Use `tool.call(json!({...}), None)` to invoke with no session context
- Pass `Some(session)` when testing session-dependent behavior
- The `call()` method is the framework-native API — avoid raw JSON-RPC request construction in unit tests

**See:** `examples/unit-test-tool.rs` for a complete example.

### Asserting Tool Annotations in `tools/list`

Tool annotations (MCP 2025-11-25) serialize with camelCase keys and are omitted when unset. Test both presence and absence to prevent wire-shape regressions:

```rust
use turul_mcp_server::prelude::*;

#[derive(McpTool, Default)]
#[tool(name = "delete_file", description = "Delete a file",
       read_only = false, destructive = true, idempotent = true, open_world = false)]
struct DeleteFileTool {
    #[param(description = "Path")]
    path: String,
}

// Tool with NO annotations — verify omission
#[derive(McpTool, Default)]
#[tool(name = "plain", description = "A plain tool")]
struct PlainTool {
    #[param(description = "Value")]
    value: String,
}

#[tokio::test]
async fn test_annotations_wire_shape() {
    // Annotated tool → camelCase keys present
    let tool = DeleteFileTool::default().to_tool();
    let json = serde_json::to_value(&tool).unwrap();
    let ann = &json["annotations"];
    assert_eq!(ann["readOnlyHint"], false);       // camelCase, not read_only_hint
    assert_eq!(ann["destructiveHint"], true);
    assert_eq!(ann["idempotentHint"], true);
    assert_eq!(ann["openWorldHint"], false);

    // Unannotated tool → annotations key absent
    let plain = PlainTool::default().to_tool();
    let json = serde_json::to_value(&plain).unwrap();
    assert!(json.get("annotations").is_none());   // omitted, not null
}
```

> **Note:** `ToolAnnotations` uses `skip_serializing_if = "Option::is_none"` on all fields, so unset hints don't appear in the JSON at all. This is distinct from resource/prompt `Annotations` (which have `audience`/`priority` fields).

## E2E Testing Architecture

E2E tests start a real HTTP server and send requests via `McpTestClient`.

```
TestServerManager::start("tools-test-server")
    → find_available_port()           # OS ephemeral port allocation
    → auto-build binary if missing    # cargo build --package <package>
    → spawn child process             # Command::new(binary_path)
    → health check loop               # POST /mcp with initialize
    → return TestServerManager { port }

McpTestClient::new(port)
    → initialize()                    # POST initialize → capture Mcp-Session-Id
    → send_initialized_notification() # POST notifications/initialized (strict mode)
    → list_tools() / call_tool()      # POST with session header
```

The `TestServerManager` auto-kills the server process on `Drop`.

**See:** `examples/e2e-test-server.rs` for a complete example.

## McpTestClient API

The test client manages session state (session ID capture, header injection) automatically.

| Method | Purpose |
|---|---|
| `initialize()` | Send `initialize` with default capabilities, capture session ID |
| `initialize_with_capabilities(caps)` | Initialize with specific client capabilities |
| `send_initialized_notification()` | Complete the strict lifecycle handshake |
| `list_tools()` / `list_resources()` / `list_prompts()` | List registered components |
| `call_tool(name, args)` | Invoke a tool with JSON arguments |
| `call_tool_with_sse(name, args)` | Invoke a tool with `Accept: text/event-stream` for progress |
| `read_resource(uri)` | Read a resource by URI |
| `get_prompt(name, args)` | Get a prompt with optional arguments |
| `connect_sse()` | Open a GET SSE stream for real-time notifications |
| `make_request(method, params, id)` | Generic JSON-RPC request |
| `send_notification(notification)` | Send a notification (no response expected) |
| `session_id()` | Get the current session ID |

**See:** `references/test-utilities-reference.md` for the full API with signatures and return types.

## Compliance Test Suite

Four compliance test modules verify MCP specification conformance:

| Module | What It Tests |
|---|---|
| **JSON-RPC format** | Request/response structure, error codes, `jsonrpc: "2.0"` |
| **Capability truthfulness** | Advertised capabilities match actual server behavior |
| **Behavioral compliance** | Lifecycle enforcement, session handshake, notification ordering |
| **Tool compliance** | `outputSchema` ↔ `structuredContent` consistency, parameter validation |

Run all compliance tests:

```bash
cargo test --test compliance
```

Run specific gates:

```bash
# Lifecycle enforcement (-32031 for pre-init access)
cargo test --test compliance test_strict_lifecycle_rejects_before_initialized

# Capability truthfulness (capabilities match support)
cargo test --test feature_tests test_tools_capability_truthfulness
cargo test --test compliance test_runtime_capability_truthfulness
```

**See:** `examples/compliance-test-custom.rs` for writing custom compliance assertions.

## SSE Testing

Test SSE streaming behavior for progress notifications and real-time events.

```rust
// turul-mcp-server v0.3
// Call tool with SSE Accept header — returns raw Response for event parsing
let response = client.call_tool_with_sse("slow_operation", json!({"input": "test"})).await?;

assert!(response.status().is_success());

// Parse SSE events from the response body
let body = response.text().await?;
for line in body.lines() {
    if line.starts_with("data: ") {
        let event_data: serde_json::Value = serde_json::from_str(&line[6..])?;
        // Verify progress notifications, final result, etc.
    }
}
```

**SSE reconnection testing**: Use `connect_sse()` to open a GET stream, then verify `Last-Event-ID` replay by disconnecting and reconnecting with the last seen event ID.

## Test Organization

The framework uses **consolidated test binaries** to minimize compilation time (43 binaries vs 155 without consolidation).

**Pattern**: Set `autotests = false` in `[package]`, then define a single `[[test]]` entry that imports all test modules:

```toml
# Cargo.toml
[package]
autotests = false   # MUST be under [package], not between sections

[[test]]
name = "all"
path = "tests/all.rs"
```

```rust
// tests/all.rs — single binary, multiple modules
#[path = "test_tools.rs"]
mod test_tools;

#[path = "test_resources.rs"]
mod test_resources;

#[path = "test_prompts.rs"]
mod test_prompts;
```

**Why**: Each test binary links the entire dependency tree. Consolidating N test files into 1 binary eliminates N-1 link steps.

## Doctest Strategy

Three tiers of documentation tests, balancing coverage vs speed:

| Tier | Attribute | When | Compile Time |
|---|---|---|---|
| **Critical API** | (none — runs by default) | Core types, builder API | < 1s per test |
| **Syntax validation** | `no_run` | Examples that need external state (DB, network) | Compile only |
| **Full integration** | `ignore` | Expensive setup, run explicitly with `--ignored` | Seconds |

**Rule**: Every ` ```rust ` block in doc comments MUST compile. Use `no_run` for examples that need external resources, `ignore` for truly expensive tests. Never use ` ```text ` for Rust code — it hides compilation errors.

## TestFixtures Helpers

`TestFixtures` provides pre-built capability objects and assertion helpers:

| Method | Returns |
|---|---|
| `resource_capabilities()` | `{"resources": {"subscribe": true, "listChanged": false}}` |
| `tools_capabilities()` | `{"tools": {"listChanged": false}}` |
| `prompts_capabilities()` | `{"prompts": {"listChanged": false}}` |
| `verify_initialization_response(result)` | Assert valid init response with `protocolVersion: "2025-11-25"` |
| `verify_error_response(result)` | Assert JSON-RPC error structure |
| `verify_resource_list_response(result)` | Assert valid `resources/list` response |
| `verify_resource_content_response(result)` | Assert valid `resources/read` response |
| `extract_tool_structured_content(result)` | Extract `structuredContent` from `tools/call` response |
| `extract_tool_content_text(result)` | Extract text content from tool result |
| `extract_tools_list(result)` | Extract tools array from `tools/list` response |

## Common Mistakes

1. **Port conflicts in parallel tests** — Always use `TestServerManager::start()` which allocates ephemeral ports via `TcpListener::bind("127.0.0.1:0")`. Never hardcode ports.

2. **`tokio::time::interval` first-tick race** — `interval()` fires immediately on the first tick. Background cleanup tasks should use `tokio::time::sleep` in a loop instead, to avoid races with TTL-sensitive tests.

3. **SQLite `:memory:` pool isolation** — Each connection in a pool gets its own in-memory database. For shared test databases, use `file:{uuid}?mode=memory&cache=shared`.

4. **Missing `Accept` header** — Streamable HTTP requires `Accept: application/json`, `text/event-stream`, or `*/*`. Omitting it causes request rejection.

5. **Forgetting `send_initialized_notification()`** — In strict lifecycle mode, the server rejects all requests before the `notifications/initialized` handshake. Always call `client.send_initialized_notification()` after `initialize()`.

6. **Testing with raw JSON instead of framework APIs** — Use `tool.call(json!({...}), None)` for unit tests and `McpTestClient` for E2E. Avoid manually constructing JSON-RPC request objects.

## Beyond This Skill

**Error handling in tests?** → See the `error-handling-patterns` skill for `McpError` variants and assertion patterns.

**Middleware testing?** → See the `middleware-patterns` skill for `McpMiddleware` trait and integration with auth/rate-limit middleware.

**Task lifecycle testing?** → See the `task-patterns` skill for task state machine assertions and `TaskRuntime` configuration.

**Lambda testing?** → See the `lambda-deployment` skill for local Lambda testing and DynamoDB test setup.

**Server configuration?** Use `McpServer::builder()`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)
