---
name: task-patterns
description: >
  This skill should be used when the user asks about "task support",
  "TaskRuntime", "TaskStorage", "task_support attribute",
  "long-running tool", "CancellationHandle", "tasks/get", "tasks/list",
  "tasks/cancel", "tasks/result", "TaskStatus", "TaskRecord",
  "TaskOutcome", "InMemoryTaskStorage", "with_task_storage",
  "task state machine", "TaskExecutor", "TokioTaskExecutor",
  "task_support = optional", "task_support = required",
  "task_support = forbidden", "with_task_runtime",
  or "task storage backend".
  Covers MCP task support for long-running tools, state machine,
  storage backends, cancellation, and capability truthfulness in
  the Turul MCP Framework (Rust).
---

# Task Patterns — Turul MCP Framework

MCP tasks enable long-running tool operations. Instead of blocking the JSON-RPC response until completion, a tool returns immediately with a task ID. The client polls for status updates and retrieves the result when ready.

## When to Use Tasks

```
How long does the tool take?
├─ < 1 second (always) ─────────→ No task support (omit task_support)
├─ Seconds to minutes ──────────→ task_support = "optional"  (client chooses)
├─ > 5 seconds (always) ────────→ task_support = "required"  (must be async)
└─ Must never run as task ──────→ task_support = "forbidden"
```

**Default: no task support.** Only add it when tools genuinely need async execution.

## Task State Machine

The MCP 2025-11-25 spec defines a strict state machine for task lifecycle:

```
                    ┌──────────────────┐
                    │                  │
         ┌─────────▼──────────┐       │
         │     Working        │───────┘ (Working → Working is INVALID)
         └─────────┬──────────┘
                   │
        ┌──────────┼──────────────────┐
        │          │                  │
        ▼          ▼                  ▼
 ┌──────────┐  ┌───────┐  ┌──────────────────┐
 │Completed │  │Failed │  │   Cancelled      │
 └──────────┘  └───────┘  └──────────────────┘
   (terminal)   (terminal)    (terminal)

         ┌─────────────────────┐
         │   InputRequired     │ ←──→ Working
         └─────────────────────┘
              │        │        │
              ▼        ▼        ▼
         Completed  Failed  Cancelled
```

**Rules:**
- `Working` → `InputRequired`, `Completed`, `Failed`, `Cancelled` (valid)
- `InputRequired` → `Working`, `Completed`, `Failed`, `Cancelled` (valid)
- `Working` → `Working` — **INVALID** (self-transition not allowed)
- `InputRequired` → `InputRequired` — **INVALID**
- `Completed`/`Failed`/`Cancelled` → anything — **INVALID** (terminal states)

The storage layer enforces these transitions. Invalid transitions return `TaskStorageError::InvalidTransition` or `TaskStorageError::TerminalState`.

## Enabling Task Support

### Step 1: Configure Task Storage on the Server

```rust
// turul-mcp-server v0.3
use turul_mcp_server::prelude::*;
use turul_mcp_task_storage::InMemoryTaskStorage;
use std::sync::Arc;

let server = McpServer::builder()
    .name("task-server")
    .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
    .tool_fn(my_slow_tool)
    .build()?;
```

**Alternatives:**
```rust
// Pre-built TaskRuntime (for custom executor or recovery timeout)
use turul_mcp_server::task::TaskRuntime;

let runtime = Arc::new(
    TaskRuntime::with_default_executor(Arc::new(InMemoryTaskStorage::new()))
        .with_recovery_timeout(600_000)  // 10 minutes
);
let server = McpServer::builder()
    .with_task_runtime(runtime)
    .build()?;

// Shortcut: in-memory with defaults
let runtime = Arc::new(TaskRuntime::in_memory());
```

### Step 2: Declare task_support on Tools

```rust
// Function macro
#[mcp_tool(name = "slow_add", description = "Add with delay", task_support = "optional")]
async fn slow_add(a: f64, b: f64) -> McpResult<f64> {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    Ok(a + b)
}

// Derive macro
#[derive(McpTool, Default)]
#[tool(name = "slow_calc", description = "Slow calc", task_support = "optional")]
struct SlowCalc {
    #[param(description = "Input value")]
    a: f64,
}

// Builder
use turul_mcp_protocol::tools::{ToolExecution, TaskSupport};

let tool = ToolBuilder::new("slow_tool")
    .description("Slow operation")
    .execution(ToolExecution { task_support: Some(TaskSupport::Optional) })
    .execute(|args| async move { Ok(serde_json::json!({"done": true})) })
    .build()?;
```

**Values:**
- `"optional"` — Client can choose sync or async execution
- `"required"` — Must always run as a task (server rejects non-task calls)
- `"forbidden"` — Must never run as a task

**See:** `examples/task-tool-declaration.rs` for all three macro patterns side by side.

## Task Storage Backends

| Backend | Feature Flag | Best For |
|---|---|---|
| `InMemoryTaskStorage` | (default) | Development, testing, single-instance |
| `SqliteTaskStorage` | `sqlite` | Single-instance production |
| `PostgresTaskStorage` | `postgres` | Multi-instance production |
| `DynamoDbTaskStorage` | `dynamodb` | Serverless / AWS Lambda |

Quick setup:

```rust
// InMemory (no feature flag needed)
use turul_mcp_task_storage::InMemoryTaskStorage;
let storage = Arc::new(InMemoryTaskStorage::new());

// SQLite (feature = "sqlite")
use turul_mcp_task_storage::SqliteTaskStorage;
let storage = Arc::new(SqliteTaskStorage::new("sqlite://tasks.db").await?);

// PostgreSQL (feature = "postgres")
use turul_mcp_task_storage::PostgresTaskStorage;
let storage = Arc::new(PostgresTaskStorage::new("postgres://localhost/mydb").await?);

// DynamoDB (feature = "dynamodb")
use turul_mcp_task_storage::DynamoDbTaskStorage;
let storage = Arc::new(DynamoDbTaskStorage::new("mcp-tasks").await?);
```

**For full backend configuration details**, see the `storage-backend-matrix` reference.

**See:** `references/task-storage-guide.md` for the `TaskStorage` trait API.

## Cancellation

The `TokioTaskExecutor` manages cancellation internally. When a client calls `tasks/cancel`:

1. The executor signals cancellation to the spawned task
2. The executor uses `tokio::select!` to race the work future against the cancel signal
3. If cancel wins, the task transitions to `Cancelled`
4. If the work completes first, cancellation is a no-op

**Tools do not directly access `CancellationHandle`** — the executor handles this transparently. Your tool code is a normal async function; the executor wraps it in cancellation logic.

```rust
// The executor internally does something like:
tokio::select! {
    outcome = work_future => { /* store result, mark Completed or Failed */ }
    _ = cancel_signal => { /* mark Cancelled */ }
}
```

**See:** `examples/cancellation-pattern.rs` and `references/task-runtime-guide.md`.

## Capability Truthfulness

The framework enforces consistency between declared capabilities and actual configuration:

1. **No runtime configured** → Server **strips `execution`** from `tools/list` responses and **rejects** task-augmented `tools/call` requests
2. **`task_support = "required"` without runtime** → **Build-time error** (`.build()` fails with a `ConfigurationError`)
3. **Runtime configured** → Full task support: `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` handlers are active

This means a client can trust the `execution` field in `tools/list` — if it's present, the server genuinely supports tasks.

## Client-Side Workflow

Brief overview (hand off to `mcp-client-patterns` for full details):

```
Client                                Server
  │                                      │
  │──── tools/call { task: {} } ────────▶│  (request async execution)
  │◀─── TaskCreated { task_id } ─────────│
  │                                      │
  │──── tasks/get { task_id } ──────────▶│  (poll status)
  │◀─── Task { status: "working" } ──────│
  │                                      │
  │──── tasks/result { task_id } ────────▶│  (blocks until terminal)
  │◀─── CallToolResult { ... } ──────────│
```

**See:** the `mcp-client-patterns` skill for full client-side task workflow patterns.

## Common Mistakes

1. **Forgetting `.with_task_storage()`** — Tools with `task_support` will have their `execution` field stripped from `tools/list` if no task runtime is configured. The server silently degrades.

2. **`task_support = "required"` without runtime** — This is a build-time error. The server's `.build()` method checks and returns `ConfigurationError` with a clear message.

3. **Expecting `CancellationHandle` in tool code** — Tools are normal async functions. The `TokioTaskExecutor` wraps your tool in cancellation logic externally. You don't need to check for cancellation signals.

4. **`Working` → `Working` transition** — This is invalid per the state machine. If you need to update progress, send `notifications/progress` instead of status transitions.

5. **Confusing `TaskOutcome::Error` vs `TaskStorageError`** — `TaskOutcome::Error` stores the result of a failed tool execution (application error). `TaskStorageError` is a storage infrastructure error (database down, invalid transition).

6. **Using `tokio::time::interval` for background cleanup** — `interval` fires immediately on the first tick, causing race conditions in tests. Use `tokio::time::sleep` in a loop instead.

## Beyond This Skill

**Storage backend configuration?** → See the `storage-backend-matrix` reference for feature flags, Cargo.toml patterns, and config structs.

**Client-side task workflows?** → See the `mcp-client-patterns` skill for `call_tool_with_task`, polling, and `TaskStatus` variants.

**Creating tools?** → See the `tool-creation-patterns` skill.

**Error handling?** → See the `error-handling-patterns` skill for `McpError` variants in tool handlers.
