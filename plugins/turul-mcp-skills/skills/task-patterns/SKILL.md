---
name: task-patterns
description: >
  This skill should be used when the user asks about "task support",
  "long-running tool", "tasks/get", "tasks/update", "tasks/cancel",
  "TaskStore", "TaskState", "InMemoryTaskStore", "with_ext_tasks",
  "ext_task_tool", "ext_task_tool_required", "Tasks extension",
  "io.modelcontextprotocol/tasks", "SEP-2663", "TaskStatus",
  "DetailedTask", "poll_interval", "notifications/tasks",
  or "task storage backend".
  Covers the MCP Tasks extension for long-running tools under 2026-07-28
  (io.modelcontextprotocol/tasks, SEP-2663) — the stateless poll-based
  lifecycle, TaskStore trait, and progressive-enhancement tool
  registration in the Turul MCP Framework (Rust). Also covers the
  frozen 2025-11-25 in-core task system (`task_support` attribute,
  `TaskRuntime`, `turul-mcp-task-storage`) for the
  `--features protocol-2025-11-25` build.
---

# Task Patterns — Turul MCP Framework

**Spec lane: MCP 2026-07-28 (current default).** Tasks moved out of core and into the `io.modelcontextprotocol/tasks` extension (SEP-2663). This skill covers that extension first; the frozen 2025-11-25 in-core system is a separate, incompatible API described at the bottom — do not mix the two on the same build.

Tasks let a tool run longer than a single request/response round trip. Instead of blocking until completion, a tool call returns a task handle; the client polls for status and retrieves the result once it lands in a terminal state.

## The 2026-07-28 Lifecycle (SEP-2663)

Redesigned around the stateless core — no session to hold a blocking call open:

- **Poll-based**: `tasks/get` (honoring the task's `pollIntervalMs` hint). **No `tasks/result`, no `tasks/list`.**
- **`tasks/update`** delivers `inputResponses` to a task sitting in `input_required`.
- **`tasks/cancel`** requests cooperative cancellation.
- **Progressive enhancement**: a tool registered via `ext_task_tool()` runs as a task only when the calling client declared the Tasks extension in `clientCapabilities.extensions`; otherwise it runs synchronously like any other tool. `ext_task_tool_required()` instead rejects non-declaring clients with `-32021` (`MissingRequiredClientCapability`, `data.requiredCapabilities.extensions`).
- **Optional push**: `notifications/tasks` status updates, subscribed via `subscriptions/listen` with `taskIds` filter fields — an alternative to polling, not a replacement for it.
- Unsolicited task handles are allowed: a `CreateTaskResult` (`resultType: "task"`) can stand in for any normal result.

## Enabling the Tasks Extension

The extension is opt-in — add `turul-mcp-ext-tasks` as a dependency and enable the `ext-tasks` feature on `turul-mcp-server` (`ext-tasks = ["protocol-2026-07-28", "dep:turul-mcp-ext-tasks"]`).

```rust
// turul-mcp-server v0.4 (feature = "ext-tasks")
use turul_mcp_server::prelude::*;
use turul_mcp_ext_tasks::InMemoryTaskStore;
use std::sync::Arc;

let server = McpServer::builder()
    .name("task-server")
    .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
    .ext_task_tool(SlowAdd)             // progressive enhancement: sync or task
    .ext_task_tool_required(SlowReport) // rejects clients that didn't declare Tasks
    .build()?;
```

`ext_task_tool()` / `ext_task_tool_required()` take a normal `McpTool` — there is no `task_support` macro attribute in this model. The tool itself doesn't know whether it's running synchronously or as a task; the server decides based on the caller's declared extension capability.

## TaskStore Trait

```rust
// turul-mcp-ext-tasks v0.4
#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError>;
    async fn get(&self, task_id: &str) -> Result<Option<TaskState>, TaskStoreError>;
    async fn complete(&self, task_id: &str, result: Value) -> Result<TaskState, TaskStoreError>;
    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError>;
    async fn cancel(&self, task_id: &str) -> Result<Option<TaskState>, TaskStoreError>;
    async fn require_input(&self, /* .. */) -> Result<TaskState, TaskStoreError>;
    async fn provide_input(&self, /* .. */) -> Result<TaskState, TaskStoreError>;
}
```

`InMemoryTaskStore` (in `turul-mcp-ext-tasks`) is the only backend shipped today — there is no SQLite/Postgres/DynamoDB `TaskStore` implementation yet, unlike the frozen 2025-11-25 `turul-mcp-task-storage` crate. Implement `TaskStore` directly for a custom backend.

## TaskStatus

```rust
pub enum TaskStatus {
    Working,
    InputRequired,
    Completed,
    Failed,
    Cancelled,
}
```

Semantics match 2025-11-25's state machine (`Working`/`InputRequired` are non-terminal; `Completed`/`Failed`/`Cancelled` are terminal), but the wire methods that observe it changed — see above.

## Client-Side Polling

```
Client                                     Server
  │                                          │
  │──── tools/call (client declared ext) ───▶│  (server elects task execution)
  │◀─── CreateTaskResult { taskId, ... } ────│  resultType: "task"
  │                                          │
  │──── tasks/get { taskId } ───────────────▶│  (poll, honoring pollIntervalMs)
  │◀─── GetTaskResult { status: working } ───│
  │        ... repeat until terminal ...     │
  │◀─── GetTaskResult { status: completed,   │
  │       result: { ... } } ─────────────────│
```

If a task lands in `input_required`, respond via `tasks/update` with `inputResponses` keyed to the outstanding `inputRequests` — not by re-calling the original tool.

**See:** the `mcp-client-patterns` skill for the client-side call surface; `ext-tasks` client support there is gated behind the `ext-tasks` client feature.

## Common Mistakes

1. **Reaching for `tasks/list`** — it doesn't exist under SEP-2663. Track task IDs at the point you created them (from the `CreateTaskResult`), or subscribe to `notifications/tasks`.
2. **Blocking on `tasks/result`** — also gone. Poll `tasks/get` or subscribe; there is no blocking-until-terminal method.
3. **Applying the old `task_support = "optional"` macro attribute and expecting `ext_task_tool()` to read it** — it doesn't. Task election under the extension is a server-side registration choice (`ext_task_tool` vs `ext_task_tool_required`), not a per-tool macro attribute.
4. **Mixing `with_task_storage()` (2025-11-25, in-core) with `with_ext_tasks()` (2026-07-28, extension)** — these are two different systems with different storage crates (`turul-mcp-task-storage` vs `turul-mcp-ext-tasks`) and different wire methods. `with_task_storage()`/`with_task_runtime()` are `#[cfg(feature = "protocol-2025-11-25")]`-gated and don't exist in a default 2026-07-28 build.
5. **Re-calling the original tool to answer `input_required`** — use `tasks/update` with `inputResponses`, not a fresh `tools/call`.

## Beyond This Skill

**Client-side task workflows?** → See the `mcp-client-patterns` skill.

**Creating the underlying tool?** → See the `tool-creation-patterns` skill.

**Error handling?** → See the `error-handling-patterns` skill for `McpError` variants, including `MissingRequiredClientCapability` (-32021).

---

## 2025-11-25 In-Core Tasks (frozen, `--no-default-features --features protocol-2025-11-25`)

This is a **different, incompatible API** — tasks in core, not an extension. It requires migration, not a compatibility shim, to move to the SEP-2663 model above.

- Server: `.with_task_storage(Arc<dyn turul_mcp_task_storage::TaskStorage>)` or `.with_task_runtime(Arc<TaskRuntime>)` (both `#[cfg(feature = "protocol-2025-11-25")]`).
- Tool declaration: `task_support = "optional" | "required" | "forbidden"` macro attribute (function macro, derive macro, or `ToolExecution { task_support: Some(TaskSupport::Optional) }` on the builder).
- Wire methods: `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` (blocks until terminal) — all four are active once a runtime is configured.
- Storage backends: `InMemoryTaskStorage` (default), `SqliteTaskStorage` (`sqlite`), `PostgresTaskStorage` (`postgres`), `DynamoDbTaskStorage` (`dynamodb`) — see `references/task-storage-guide.md` for the `TaskStorage` trait and per-backend config structs.
- Cancellation: the `TokioTaskExecutor` races the work future against a cancel signal internally via `tokio::select!`; tools do not touch `CancellationHandle` directly. See `references/task-runtime-guide.md`.
- Capability truthfulness: no runtime configured → server strips `execution` from `tools/list` and rejects task-augmented `tools/call`; `task_support = "required"` without a runtime is a build-time `ConfigurationError`.
- State machine: `Working`/`InputRequired` ⇄ each other, both → any terminal state (`Completed`/`Failed`/`Cancelled`); self-transitions (`Working`→`Working`) are invalid and rejected by the storage layer as `TaskStorageError::InvalidTransition`.
